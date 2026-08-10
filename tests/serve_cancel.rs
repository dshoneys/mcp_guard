use anyhow::Result;
use mcp_guard::config::{AuditConfig, Config};
use mcp_guard::contracts::{AlertSink, ScanReport, Scanner, WatchReport, Watcher};
use mcp_guard::serve;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

struct CountingScanner {
    n: Arc<AtomicUsize>,
}

impl Scanner for CountingScanner {
    async fn scan(&self, _cfg: &Config, _extra_ports: &[u16]) -> Result<ScanReport> {
        self.n.fetch_add(1, Ordering::SeqCst);
        Ok(ScanReport {
            host: "127.0.0.1".into(),
            scanned_at: "t".into(),
            findings: vec![],
        })
    }
}

struct NopWatcher;

impl Watcher for NopWatcher {
    fn watch(&self, _cfg: &Config) -> Result<WatchReport> {
        Ok(WatchReport {
            watched_at: "t".into(),
            ports: vec![],
            alert_count: 0,
        })
    }
}

struct NopSink;

impl AlertSink for NopSink {
    fn append(
        &self,
        _cfg: &AuditConfig,
        _kind: &str,
        _detail: serde_json::Value,
    ) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn cancel_stops_loop_before_many_ticks() {
    let scans = Arc::new(AtomicUsize::new(0));
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_bg = Arc::clone(&cancel);
    let scans_bg = Arc::clone(&scans);

    let mut cfg = Config::default();
    cfg.serve.interval_secs = 30;
    cfg.audit.path = std::env::temp_dir().join("mcp-guard-cancel-unused.jsonl");

    let handle = tokio::spawn(async move {
        let scanner = CountingScanner { n: scans_bg };
        serve::run_with_cancel(
            &cfg,
            false,
            &scanner,
            &NopWatcher,
            &NopSink,
            Some(cancel_bg),
        )
        .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    cancel.store(true, Ordering::SeqCst);
    handle.await.unwrap().unwrap();

    let n = scans.load(Ordering::SeqCst);
    assert!(n >= 1, "expected at least one tick, got {n}");
    assert!(n < 5, "cancel should stop early, got {n} ticks");
}
