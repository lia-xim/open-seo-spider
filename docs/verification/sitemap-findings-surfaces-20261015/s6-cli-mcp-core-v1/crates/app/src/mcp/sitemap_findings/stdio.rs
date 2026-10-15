//! Findings-only cancellation pump. Legacy calls retain serial dispatch.
use super::{Bridge, NAME, error, valid_rpc_id};
use crate::mcp::MAX_STDIO_REQUEST_BYTES as MAX;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

pub(super) enum Frame {
    Eof,
    Complete(Vec<u8>),
    Oversized,
}

pub(super) async fn read_frame<R: AsyncRead + Unpin>(
    input: &mut BufReader<R>,
    partial: &mut Vec<u8>,
) -> std::io::Result<Frame> {
    // read_until is cancellation-safe: already appended bytes survive select!.
    // A resumed partial line receives only its REMAINING sentinel allowance.
    let remaining = MAX + 1 - partial.len();
    let read = (&mut *input)
        .take(remaining as u64)
        .read_until(b'\n', partial)
        .await?;
    if partial.len() > MAX {
        return Ok(Frame::Oversized);
    }
    if read == 0 && partial.is_empty() {
        return Ok(Frame::Eof);
    }
    Ok(Frame::Complete(std::mem::take(partial)))
}
async fn write_response<W: AsyncWrite + Unpin>(
    output: &mut W,
    response: Value,
) -> anyhow::Result<()> {
    let bytes = tokio::task::spawn_blocking(move || serde_json::to_vec(&response)).await??;
    output.write_all(&bytes).await?;
    output.write_all(b"\n").await?;
    output.flush().await?;
    Ok(())
}
fn findings_request(request: &Value) -> bool {
    request["jsonrpc"] == "2.0"
        && request["method"] == "tools/call"
        && request["params"]["name"] == NAME
        && request.get("id").is_some()
}
fn matching_cancel(request: &Value, id: &Value) -> bool {
    request["jsonrpc"] == "2.0"
        && request["method"] == "notifications/cancelled"
        && request.get("id").is_none()
        && request["params"].get("requestId") == Some(id)
}

pub(in crate::mcp) async fn stdio_stream<R, W>(
    bridge: &Bridge,
    input: R,
    mut output: W,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut input = BufReader::new(input);
    let mut partial = Vec::new();
    let mut pending = None;
    loop {
        let bytes = match pending.take() {
            Some(bytes) => bytes,
            None => match read_frame(&mut input, &mut partial).await? {
                Frame::Eof => break,
                Frame::Oversized => {
                    write_response(
                        &mut output,
                        error(Value::Null, -32600, "Request exceeds 8 MiB"),
                    )
                    .await?;
                    break;
                }
                Frame::Complete(bytes) => bytes,
            },
        };
        let request: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                write_response(&mut output, error(Value::Null, -32700, "Invalid JSON")).await?;
                continue;
            }
        };
        drop(bytes);
        if !findings_request(&request) {
            if let Some(response) = bridge.dispatch(request).await {
                write_response(&mut output, response).await?;
            }
            continue;
        }
        let active_id = request["id"].clone();
        if !valid_rpc_id(&active_id) {
            write_response(
                &mut output,
                error(
                    Value::Null,
                    -32600,
                    "Finding RPC id must be an integer or at most 1024 UTF-8 bytes",
                ),
            )
            .await?;
            continue;
        }
        // This future owns the actual reqwest send/body future. Dropping it on
        // matching cancellation or EOF closes that request, with no detached task.
        let mut active = Box::pin(bridge.dispatch(request));
        let response = loop {
            tokio::select! {
                result = &mut active => break result,
                frame = read_frame(&mut input,&mut partial) => {
                    match frame? {
                        Frame::Eof => return Ok(()),
                        Frame::Oversized => { write_response(&mut output,error(Value::Null,-32600,"Request exceeds 8 MiB")).await?; return Ok(()); }
                        Frame::Complete(bytes) => {
                            let value: Value = match serde_json::from_slice(&bytes) {
                                Ok(value) => value,
                                Err(_) => { write_response(&mut output,error(Value::Null,-32700,"Invalid JSON")).await?; continue; }
                            };
                            if matching_cancel(&value,&active_id) {
                                break Some(error(active_id.clone(),-32800,"Request cancelled"));
                            }
                            if pending.is_none() { pending=Some(bytes); }
                            else if let Some(id)=value.get("id") {
                                let id=if valid_rpc_id(id) { id.clone() } else { Value::Null };
                                write_response(&mut output,error(id,-32000,"One request is already pending while findings are active")).await?;
                            }
                            // A full pending slot never prevents reading a matching
                            // cancel. Additional notifications are not queued.
                        }
                    }
                }
            }
        };
        drop(active);
        if let Some(response) = response {
            write_response(&mut output, response).await?;
        }
    }
    Ok(())
}
