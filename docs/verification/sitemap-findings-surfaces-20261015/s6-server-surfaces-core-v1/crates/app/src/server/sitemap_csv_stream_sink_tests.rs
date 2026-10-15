//! AUTHORED / NOT COMPILED / NOT RUN. Sink controls only, not stored-route proof.
use super::*;

#[tokio::test]
async fn csv_first_count_pass_restores_actual_budget_cause_when_formatter_erases_io_source() {
    // A sink regression, not proof of a stored route or a real Core session.
    struct ErasingFormatter;
    impl CsvFormatter for ErasingFormatter {
        fn pass<W: Write>(&self, writer: &mut W, _: &mut dyn FnMut()) -> anyhow::Result<u64> {
            let block = [b'q'; CHUNK_BYTES];
            for _ in 0..membership::MAX_CSV_BYTES / CHUNK_BYTES {
                writer.write_all(&block)?;
            }
            writer
                .write_all(b"+")
                .map_err(|_| anyhow::anyhow!("formatter discarded IO cause"))?;
            Ok(0)
        }
    }
    let registry = ExportRegistry::default();
    let (changed, _) = watch::channel(false);
    let stop = Stop {
        control: Default::default(),
        changed,
    };
    let (sender, _receiver) = mpsc::channel(QUEUE_CHUNKS);
    let (progress, updates) = watch::channel(Progress {
        ready_at: None,
        last_sent: Instant::now(),
    });
    let (ready_sender, mut ready_receiver) = oneshot::channel::<Result<Ready<()>, Failure>>();
    let mut ready = Some(ready_sender);
    let result = produce_session(
        &registry,
        &stop,
        sender,
        progress,
        &mut ready,
        tokio::runtime::Handle::current(),
        (),
        ErasingFormatter,
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("first pass must fail"),
    };
    assert_eq!(
        error.downcast_ref::<Failure>().unwrap().code,
        "sitemap_csv_byte_budget"
    );
    assert!(matches!(
        ready_receiver.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    assert!(updates.borrow().ready_at.is_none());
    assert_eq!(registry.snapshot().completed_passes, 0);
    assert_eq!(registry.snapshot().prefix_bytes, 0);
}

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
