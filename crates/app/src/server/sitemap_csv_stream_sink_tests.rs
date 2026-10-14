//! AUTHORED / NOT COMPILED / NOT RUN. Sink controls only, not stored-route proof.
use super::*;

#[test]
fn counting_hash_sink_counts_actual_sixty_four_mib_and_refuses_next_byte() {
    let block = vec![b'q'; CHUNK_BYTES];
    let mut sink = CountHash::new();
    let mut independent = Sha256::new();
    for _ in 0..membership::MAX_CSV_BYTES / CHUNK_BYTES {
        sink.write_all(&block).unwrap();
        independent.update(&block);
    }
    let exact = sink.receipt();
    assert_eq!(exact.length, 67_108_864);
    assert_eq!(exact.sha256, format!("{:x}", independent.finalize()));
    assert!(sink.write_all(b"+").is_err());
    assert_eq!(sink.receipt(), exact);
    assert_eq!(
        sink.failure.as_ref().unwrap().code,
        "sitemap_csv_byte_budget"
    );
}

#[tokio::test]
async fn chunk_sink_keeps_positive_tail_for_exact_multiples_and_flush() {
    for length in [1, CHUNK_BYTES, CHUNK_BYTES * 2, CHUNK_BYTES * 2 + 1] {
        let bytes: Vec<_> = (0..length).map(|index| (index % 251) as u8).collect();
        let expected = bytes.clone();
        let (sender, mut receiver) = mpsc::channel(QUEUE_CHUNKS);
        let (changed, _) = watch::channel(false);
        let stop = Stop {
            control: Default::default(),
            changed,
        };
        let (progress, _) = watch::channel(Progress {
            ready_at: Some(Instant::now()),
            last_sent: Instant::now(),
        });
        let counters = Arc::new(Counters::default());
        let check_counters = counters.clone();
        let runtime = tokio::runtime::Handle::current();
        let worker = tokio::task::spawn_blocking(move || {
            let mut writer = ChunkTailWriter {
                count: CountHash::new(),
                tail: Vec::with_capacity(CHUNK_BYTES),
                sender,
                stop,
                runtime,
                progress,
                counters,
                limits: Default::default(),
                fault: Default::default(),
            };
            writer.write_all(&bytes).unwrap();
            writer.flush().unwrap();
            let receipt = writer.count.receipt();
            (writer.tail, receipt)
        });
        let mut prefix = Vec::new();
        while let Some(chunk) = receiver.recv().await {
            assert!(chunk.len() <= CHUNK_BYTES);
            prefix.extend_from_slice(&chunk);
        }
        let (tail, receipt) = worker.await.unwrap();
        assert!(!tail.is_empty());
        assert!(tail.len() <= CHUNK_BYTES);
        assert!(
            prefix.len() < length,
            "last positive byte must stay behind join"
        );
        prefix.extend_from_slice(&tail);
        assert_eq!(prefix, expected);
        assert_eq!(receipt.length, length as u64);
        assert_eq!(receipt.sha256, format!("{:x}", Sha256::digest(&prefix)));
        assert!(check_counters.peak_queued.load(Ordering::SeqCst) <= QUEUE_CHUNKS);
    }
}

#[tokio::test]
async fn cancelled_sink_retains_typed_cause_despite_unchanged_formatter_budget_label() {
    let (sender, _receiver) = mpsc::channel(QUEUE_CHUNKS);
    let (changed, _) = watch::channel(false);
    let stop = Stop {
        control: Default::default(),
        changed,
    };
    stop.cancel();
    let (progress, _) = watch::channel(Progress {
        ready_at: Some(Instant::now()),
        last_sent: Instant::now(),
    });
    let mut writer = ChunkTailWriter {
        count: CountHash::new(),
        tail: Vec::with_capacity(CHUNK_BYTES),
        sender,
        stop,
        runtime: tokio::runtime::Handle::current(),
        progress,
        counters: Arc::new(Counters::default()),
        limits: Default::default(),
        fault: Default::default(),
    };
    let formatter_error = membership::write_csv_header(&mut writer).unwrap_err();
    assert_eq!(
        formatter_error.code, "sitemap_csv_byte_budget",
        "old formatter behavior remains byte/error compatible"
    );
    let actual = writer.count.failure.unwrap();
    assert_eq!(actual.code, "sitemap_export_cancelled");
    assert_eq!(actual.status(), 503);
    let deadline = SitemapReadControl::default();
    deadline.expire();
    assert_eq!(classify(deadline.checkpoint().unwrap_err()).status(), 504);
}
