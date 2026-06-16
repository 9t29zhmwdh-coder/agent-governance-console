use agc_core::{AuditLog, ConsoleConfig, PolicyEngine, TraceStore};

fn main() {
    let cfg = ConsoleConfig::default_local();
    println!("Agent Governance Console v{}", env!("CARGO_PKG_VERSION"));
    println!("API bind       : {}", cfg.api_bind);
    println!("Telemetry      : {}", cfg.telemetry.enabled);
    println!("Audit export   : {:?}", cfg.audit_export_path);

    let _traces = TraceStore::new();
    let _audit  = AuditLog::new();
    let _policy = PolicyEngine::new();

    println!("\nAll subsystems initialised. Run `agc-api` to start the REST API.");
}
