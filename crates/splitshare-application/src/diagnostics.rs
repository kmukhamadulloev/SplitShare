//! Bounded, process-local host diagnostics. No HTTP or filesystem dependency.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Warn,
    Info,
    Debug,
}
#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub id: u64,
    pub timestamp_ms: u64,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: BTreeMap<String, String>,
}
#[derive(Serialize)]
pub struct LogConfig {
    pub level: Option<LogLevel>,
    pub available: bool,
    pub capacity: usize,
}
type Control = Arc<dyn Fn(LogLevel) -> Result<(), &'static str> + Send + Sync>;
struct History {
    next: u64,
    entries: VecDeque<LogEntry>,
}
struct Settings {
    level: Option<LogLevel>,
    control: Option<Control>,
}
struct Inner {
    history: Mutex<History>,
    settings: Mutex<Settings>,
    changed: broadcast::Sender<()>,
}
#[derive(Clone)]
pub struct Diagnostics(Arc<Inner>);
impl Default for Diagnostics {
    fn default() -> Self {
        let (changed, _) = broadcast::channel(64);
        Self(Arc::new(Inner {
            history: Mutex::new(History {
                next: 1,
                entries: VecDeque::new(),
            }),
            settings: Mutex::new(Settings {
                level: None,
                control: None,
            }),
            changed,
        }))
    }
}
impl Diagnostics {
    pub fn install_control(&self, control: Control) {
        self.0.settings.lock().unwrap().control = Some(control);
    }
    pub fn config(&self) -> LogConfig {
        let s = self.0.settings.lock().unwrap();
        LogConfig {
            level: s.level,
            available: s.control.is_some(),
            capacity: 500,
        }
    }
    pub fn configure(&self, level: LogLevel) -> Result<(), &'static str> {
        {
            let mut settings = self.0.settings.lock().unwrap();
            settings
                .control
                .as_ref()
                .ok_or("Logging control is unavailable.")?(level)?;
            settings.level = Some(level);
        }
        let _ = self.0.changed.send(());
        Ok(())
    }
    pub fn record(
        &self,
        level: &str,
        target: &str,
        message: &'static str,
        fields: BTreeMap<String, String>,
    ) {
        let mut history = self.0.history.lock().unwrap();
        let entry = LogEntry {
            id: history.next,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            level: level.chars().take(8).collect(),
            target: target.chars().take(120).collect(),
            message: message.into(),
            fields: fields
                .into_iter()
                .take(20)
                .map(|(k, v)| (k.chars().take(40).collect(), v.chars().take(256).collect()))
                .collect(),
        };
        history.next += 1;
        if history.entries.len() == 500 {
            history.entries.pop_front();
        }
        history.entries.push_back(entry);
        drop(history);
        let _ = self.0.changed.send(());
    }
    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.0
            .history
            .lock()
            .unwrap()
            .entries
            .iter()
            .cloned()
            .collect()
    }
    pub fn clear(&self) {
        self.0.history.lock().unwrap().entries.clear();
        let _ = self.0.changed.send(());
    }
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.0.changed.subscribe()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_history_and_runtime_control() {
        let d = Diagnostics::default();
        assert!(d.configure(LogLevel::Debug).is_err());
        d.install_control(Arc::new(|_| Ok(())));
        d.configure(LogLevel::Debug).unwrap();
        assert_eq!(d.config().level, Some(LogLevel::Debug));
        for _ in 0..510 {
            d.record(
                "INFO",
                "test",
                "message",
                BTreeMap::from([("field".into(), "x".repeat(1000))]),
            );
        }
        let rows = d.snapshot();
        assert_eq!(rows.len(), 500);
        assert_eq!(rows[0].id, 11);
        assert_eq!(rows[0].fields["field"].len(), 256);
        d.clear();
        assert!(d.snapshot().is_empty());
        d.record("INFO", "test", "message", BTreeMap::new());
        assert_eq!(d.snapshot()[0].id, 511);
    }
}
