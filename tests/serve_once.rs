use anyhow::Result;
use mcp_guard::config::{AuditConfig, Config};
use mcp_guard::contracts::{AlertSink, ScanReport, Scanner, WatchReport, Watcher};
use mcp_guard::serve;
use std::sync::{Arc, Mutex};

struct MockScanner;

impl Scanner for MockScanner {
    async fn scan(&self, _cfg: &Config, _extra_ports: &[u16]) -> Result<ScanReport> {
        Ok(ScanReport {
            host: "127.0.0.1".into(),
            scanned_at: "t".into(),
            findings: vec![],
        })
    }
}

struct MockWatcher;

impl Watcher for MockWatcher {
    fn watch(&self, _cfg: &Config) -> Result<WatchReport> {
        Ok(WatchReport {
            watched_at: "t".into(),
            ports: vec![],
            alert_count: 0,
        })
    }
}

struct RecordingSink {
    kinds: Arc<Mutex<Vec<String>>>,
}

impl AlertSink for RecordingSink {
    fn append(
        &self,
        _cfg: &AuditConfig,
        kind: &str,
        _detail: serde_json::Value,
    ) -> Result<()> {
        self.kinds.lock().unwrap().push(kind.to_string());
        Ok(())
    }
}

#[tokio::test]
async fn serve_once_emits_scan_and_watch_kinds() {
    let kinds = Arc::new(Mutex::new(Vec::new()));
    let sink = RecordingSink {
        kinds: Arc::clone(&kinds),
    };
    let mut cfg = Config::default();
    cfg.audit.path = std::env::temp_dir().join("mcp-guard-serve-once-unused.jsonl");

    serve::run_with(&cfg, true, &MockScanner, &MockWatcher, &sink)
        .await
        .unwrap();

    let recorded = kinds.lock().unwrap().clone();
    assert!(recorded.contains(&"scan".to_string()));
    assert!(recorded.contains(&"watch".to_string()));
}
