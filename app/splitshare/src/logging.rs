//! Tracing adapter. Only approved messages and structured diagnostic fields reach the browser.
use splitshare_application::diagnostics::{Diagnostics, LogLevel};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};
use tracing::{
    Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{
    Layer,
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
};
static DIAGNOSTICS: OnceLock<Diagnostics> = OnceLock::new();
pub fn diagnostics() -> Diagnostics {
    DIAGNOSTICS.get_or_init(Diagnostics::default).clone()
}
#[derive(Default, Clone)]
struct Fields(BTreeMap<String, String>);
impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let name = field.name();
        if !matches!(
            name,
            "message"
                | "transfer_id"
                | "expected_bytes"
                | "conflict_policy"
                | "concurrency_limit"
                | "stage"
                | "failure"
                | "cause"
                | "input_error_kind"
                | "kind"
                | "bytes_written"
                | "elapsed_ms"
                | "queue_wait_ms"
                | "idle_timeout_seconds"
                | "timeout_seconds"
                | "worker_panicked"
                | "worker_cancelled"
                | "state"
                | "reason"
                | "sharing"
        ) {
            return;
        }
        let text = format!("{value:?}");
        if text.len() > 256 {
            return;
        }
        if name == "message" {
            self.0.insert(name.into(), text);
            return;
        }
        let text = text.trim_matches('"');
        if text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_(). :-".contains(c))
        {
            self.0.insert(name.into(), text.into());
        }
    }
}
fn approved_message(value: &str) -> &'static str {
    match value {
        "Upload accepted" => "Upload accepted",
        "Upload worker started" => "Upload worker started",
        "Upload completed" => "Upload completed",
        "Upload failed before publication" => "Upload failed before publication",
        "Upload cancelled" => "Upload cancelled",
        "Upload cancellation requested" => "Upload cancellation requested",
        "Upload handler ended before publication" => "Upload handler ended before publication",
        "Upload queue deadline exceeded" => "Upload queue deadline exceeded",
        "Upload worker terminated unexpectedly" => "Upload worker terminated unexpectedly",
        "Upload progress" => "Upload progress",
        "Upload publication started" => "Upload publication started",
        "Storage operation failed" => "Storage operation failed",
        "Partial upload cleanup failed" => "Partial upload cleanup failed",
        "Upload published" => "Upload published",
        "SplitShare started" => "SplitShare started",
        "Server stopped" => "Server stopped",
        "Host settings updated" => "Host settings updated",
        "Host folder/network setup applied; old sessions revoked" => {
            "Host folder/network setup applied; old sessions revoked"
        }
        "Share session joined" => "Share session joined",
        "Share token rotated; remote sessions revoked" => {
            "Share token rotated; remote sessions revoked"
        }
        "Filesystem mutation completed" => "Filesystem mutation completed",
        "Logging level changed" => "Logging level changed",
        "Browser open failed" => "Browser open failed",
        "Desktop action failed" => "Desktop action failed",
        "HTTP connection task failed" => "HTTP connection task failed",
        "Saved shared folder is unavailable; choose a folder in host settings" => {
            "Saved shared folder is unavailable; choose a folder in host settings"
        }
        "Host setup file is invalid or unavailable; refusing to overwrite it" => {
            "Host setup file is invalid or unavailable; refusing to overwrite it"
        }
        "Could not restore host setup after shutdown failure" => {
            "Could not restore host setup after shutdown failure"
        }
        "Host setup failed; previous listener and sandbox restored" => {
            "Host setup failed; previous listener and sandbox restored"
        }
        "Tray unavailable; server continues. Use --no-tray for headless operation" => {
            "Tray unavailable; server continues. Use --no-tray for headless operation"
        }
        "Unable to persist host settings" => "Unable to persist host settings",
        "Native tray ready" => "Native tray ready",
        "Could not disable TCP coalescing; small requests may be delayed" => {
            "Could not disable TCP coalescing; small requests may be delayed"
        }
        "HTTP connection task failed during shutdown" => {
            "HTTP connection task failed during shutdown"
        }
        _ => "Diagnostic event (message omitted)",
    }
}
struct Capture(Diagnostics);
impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Capture {
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::Id,
        ctx: Context<'_, S>,
    ) {
        let mut fields = Fields::default();
        attrs.record(&mut fields);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(fields);
        }
    }
    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        if !event.metadata().target().starts_with("splitshare") {
            return;
        }
        let mut fields = Fields::default();
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(values) = span.extensions().get::<Fields>() {
                    fields.0.extend(values.0.clone());
                }
            }
        }
        event.record(&mut fields);
        let message = approved_message(&fields.0.remove("message").unwrap_or_default());
        self.0.record(
            event.metadata().level().as_str(),
            event.metadata().target(),
            message,
            fields.0,
        );
    }
}
pub fn init() {
    let diagnostics = diagnostics();
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "splitshare=info".into());
    let (filter, handle) = tracing_subscriber::reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(Capture(diagnostics.clone()))
        .init();
    diagnostics.install_control(Arc::new(move |level| {
        let filter = match level {
            LogLevel::Warn => "warn,splitshare=warn",
            LogLevel::Info => "warn,splitshare=info",
            LogLevel::Debug => "warn,splitshare=debug",
        };
        handle
            .reload(tracing_subscriber::EnvFilter::new(filter))
            .map_err(|_| "Could not change logging level.")
    }));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_excludes_sensitive_fields_and_unapproved_messages() {
        let d = Diagnostics::default();
        let subscriber = tracing_subscriber::registry().with(Capture(d.clone()));
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!(
                "upload",
                transfer_id = "123",
                key = "SECRET",
                path = "/private"
            );
            let _guard = span.enter();
            tracing::warn!(target:"splitshare_test",bytes_written=3,error="SECRET",cause="/private", "Upload failed before publication");
            tracing::warn!(target:"splitshare_test","secret {}","SECRET");
        });
        let rows = d.snapshot();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].fields["bytes_written"], "3");
        assert_eq!(rows[0].fields["transfer_id"], "123");
        let encoded = serde_json::to_string(&rows).unwrap();
        assert!(!encoded.contains("SECRET"));
        assert!(!encoded.contains("/private"));
        assert!(encoded.contains("message omitted"));
    }
}
