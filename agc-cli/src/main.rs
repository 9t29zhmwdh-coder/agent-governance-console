use agc_azure::{GraphClient, ManagedIdentityCredential, MonitorIngestClient};
use agc_core::{AuditLog, AuditRecord, ConsoleConfig, GovernancePolicy, PolicyEngine, TraceStore};
use clap::{Parser, Subcommand};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Parser)]
#[command(name = "agc-cli", about = "Agent Governance Console: command-line interface")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Azure integration commands (Managed Identity auth, Monitor, Graph)
    Azure {
        #[command(subcommand)]
        action: AzureCommand,
    },
    /// Policy DSL commands (YAML/JSON validation, Rego export)
    Policy {
        #[command(subcommand)]
        action: PolicyCommand,
    },
    /// Export Microsoft Sentinel analytics rule templates for the AGC audit table
    Sentinel {
        #[command(subcommand)]
        action: SentinelCommand,
    },
    /// Load-test a running agc-api instance
    Bench {
        #[command(subcommand)]
        action: BenchCommand,
    },
}

#[derive(Subcommand)]
enum BenchCommand {
    /// Load-test POST /api/v1/traces and report ingest latency percentiles
    /// against the ROADMAP.md SLA target (p99 < 10ms at 1K spans/s).
    /// Sends `rate` requests per second, evenly spaced (one every
    /// 1/rate seconds, not a synchronous per-second burst -- see
    /// docs/performance.md for why that distinction matters), for
    /// `duration_secs` seconds, against an already-running agc-api
    /// instance -- run `cargo run -p agc-api` (or a deployed one) in
    /// another terminal first.
    Ingest {
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        url: String,
        #[arg(long, default_value = "agc-bench")]
        tenant: String,
        #[arg(long, default_value_t = 1000)]
        rate: u32,
        #[arg(long, default_value_t = 10)]
        duration_secs: u64,
    },
}

#[derive(Subcommand)]
enum SentinelCommand {
    /// Write the built-in Sentinel analytics rule templates to disk
    Export {
        /// The Log Analytics custom table name (scripts/azure_setup.sh
        /// creates AGCAudit_CL by default)
        #[arg(long, default_value = "AGCAudit_CL")]
        table: String,
        /// "kql" writes one .kql file per rule; "arm" writes one ARM
        /// template deploying all of them via
        /// `az deployment group create`
        #[arg(long, default_value = "kql")]
        format: String,
        #[arg(long, default_value = ".")]
        output_dir: std::path::PathBuf,
    },
}

#[derive(Subcommand)]
enum PolicyCommand {
    /// Parse a policy file (YAML or JSON) and report whether it's valid
    Validate {
        /// Path to a policy file
        file: std::path::PathBuf,
    },
    /// Parse a policy file and print a best-effort Rego stub for it
    ToRego {
        /// Path to a policy file
        file: std::path::PathBuf,
    },
}

#[derive(Subcommand)]
enum AzureCommand {
    /// List Entra ID app registrations tagged `agc-agent`
    ListAgents {
        /// User-assigned managed identity client ID (omit for system-assigned)
        #[arg(long)]
        client_id: Option<String>,
    },
    /// Push an exported audit NDJSON file to an Azure Monitor DCR
    ///
    /// Run `scripts/export_audit.sh ndjson` against a running agc-api
    /// first to produce the input file.
    PushAudit {
        /// Path to an NDJSON file (one AuditRecord per line)
        #[arg(long)]
        file: std::path::PathBuf,
        /// Data Collection Endpoint, e.g. https://<name>.<region>-1.ingest.monitor.azure.com
        #[arg(long)]
        dce_endpoint: String,
        /// The DCR's immutable ID (dcr-...), not its display name
        #[arg(long)]
        dcr_id: String,
        /// Custom table stream name
        #[arg(long, default_value = "Custom-AGCAudit_CL")]
        stream: String,
        /// User-assigned managed identity client ID (omit for system-assigned)
        #[arg(long)]
        client_id: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let result = match Cli::parse().command {
        None => {
            print_info();
            Ok(())
        }
        Some(Command::Azure { action }) => run_azure_command(action).await,
        Some(Command::Policy { action }) => run_policy_command(action),
        Some(Command::Sentinel { action }) => run_sentinel_command(action),
        Some(Command::Bench { action }) => run_bench_command(action).await,
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn print_info() {
    let cfg = ConsoleConfig::default_local();
    println!("Agent Governance Console v{}", env!("CARGO_PKG_VERSION"));
    println!("API bind       : {}", cfg.api_bind);
    println!("Telemetry      : {}", cfg.telemetry.enabled);
    println!("Audit export   : {:?}", cfg.audit_export_path);

    let _traces = TraceStore::new();
    let _audit = AuditLog::new();
    let _policy = PolicyEngine::new();

    println!("\nAll subsystems initialised. Run `agc-api` to start the REST API.");
    println!("Run `agc-cli azure --help` for Azure integration commands.");
    println!("Run `agc-cli policy --help` for policy DSL commands.");
    println!("Run `agc-cli sentinel --help` for Microsoft Sentinel export commands.");
    println!("Run `agc-cli bench --help` for load-testing commands.");
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn run_sentinel_command(action: SentinelCommand) -> Result<(), BoxError> {
    match action {
        SentinelCommand::Export { table, format, output_dir } => {
            std::fs::create_dir_all(&output_dir)?;
            let rules = agc_core::sentinel_builtin_rules(&table);

            match format.as_str() {
                "kql" => {
                    for rule in &rules {
                        let path = output_dir.join(format!("{}.kql", sanitize_filename(rule.name)));
                        std::fs::write(&path, format!("// {}\n// {}\n// Severity: {}\n\n{}\n", rule.name, rule.description, rule.severity, rule.to_kql()))?;
                        println!("Wrote {}", path.display());
                    }
                }
                "arm" => {
                    let template = serde_json::json!({
                        "$schema": "https://schema.management.azure.com/schemas/2019-04-01/deploymentTemplate.json#",
                        "contentVersion": "1.0.0.0",
                        "resources": rules.iter().map(|r| r.to_arm_resource()).collect::<Vec<_>>(),
                    });
                    let path = output_dir.join("agc-sentinel-rules.json");
                    std::fs::write(&path, serde_json::to_string_pretty(&template)?)?;
                    println!("Wrote {} ({} rule(s))", path.display(), rules.len());
                }
                other => return Err(format!("unknown --format '{other}', expected 'kql' or 'arm'").into()),
            }
            Ok(())
        }
    }
}

fn bench_span_json(tenant: &str) -> serde_json::Value {
    serde_json::json!({
        "span_id": uuid::Uuid::new_v4(),
        "trace_id": uuid::Uuid::new_v4(),
        "parent_span_id": null,
        "agent_id": format!("{tenant}-bench-agent"),
        "operation": "bench_call",
        "level": "info",
        "started_at": chrono::Utc::now().to_rfc3339(),
        "ended_at": null,
        "attributes": {},
    })
}

async fn run_bench_command(action: BenchCommand) -> Result<(), BoxError> {
    match action {
        BenchCommand::Ingest { url, tenant, rate, duration_secs } => {
            let client = reqwest::Client::new();
            let traces_url = format!("{}/api/v1/traces", url.trim_end_matches('/'));
            println!("Bench: POST {traces_url}, {rate} req/s for {duration_secs}s (tenant '{tenant}')");

            let total_requests = rate as u64 * duration_secs;
            let mut handles = Vec::with_capacity(total_requests as usize);

            // Spread requests evenly across the whole run (one every
            // 1/rate seconds) rather than firing `rate` requests in one
            // synchronous burst per second. A synchronous burst was tried
            // first and produced misleadingly high p99s (~30ms at 1000
            // req/s): it makes ~1000 requests contend for the same
            // per-tenant lock at the exact same instant, which a real
            // steady arrival rate of 1000/s never does (see
            // docs/performance.md for the full investigation).
            let interval = std::time::Duration::from_secs_f64(1.0 / rate as f64);
            let run_start = std::time::Instant::now();
            for i in 0..total_requests {
                let target = run_start + interval * i as u32;
                let now = std::time::Instant::now();
                if target > now {
                    tokio::time::sleep(target - now).await;
                }
                let client = client.clone();
                let traces_url = traces_url.clone();
                let tenant = tenant.clone();
                handles.push(tokio::spawn(async move {
                    let body = bench_span_json(&tenant);
                    let start = std::time::Instant::now();
                    let result = client.post(&traces_url).header("X-Tenant-Id", &tenant).json(&body).send().await;
                    let ok = result.map(|r| r.status().is_success()).unwrap_or(false);
                    (start.elapsed(), ok)
                }));
                if (i + 1) % rate as u64 == 0 {
                    println!("  second {}/{duration_secs}: {rate} requests sent", (i + 1) / rate as u64);
                }
            }

            let mut latencies_us: Vec<u64> = Vec::with_capacity(total_requests as usize);
            let mut errors: u64 = 0;
            for handle in handles {
                match handle.await {
                    Ok((elapsed, true)) => latencies_us.push(elapsed.as_micros() as u64),
                    _ => errors += 1,
                }
            }

            latencies_us.sort_unstable();
            let report = agc_core::LatencyReport::from_sorted_micros(&latencies_us, errors);
            println!();
            println!("Results: {} successful, {} error(s)", report.count, report.errors);
            println!("  p50: {:.2}ms", report.p50_us as f64 / 1000.0);
            println!("  p95: {:.2}ms", report.p95_us as f64 / 1000.0);
            println!("  p99: {:.2}ms", report.p99_ms());
            println!("  max: {:.2}ms", report.max_us as f64 / 1000.0);

            const SLA_P99_MS: f64 = 10.0;
            if report.p99_ms() >= SLA_P99_MS {
                eprintln!("SLA NOT MET: p99 {:.2}ms >= {SLA_P99_MS}ms target", report.p99_ms());
                std::process::exit(1);
            }
            println!("SLA MET: p99 {:.2}ms < {SLA_P99_MS}ms target", report.p99_ms());
            Ok(())
        }
    }
}

fn run_policy_command(action: PolicyCommand) -> Result<(), BoxError> {
    match action {
        PolicyCommand::Validate { file } => {
            let content = std::fs::read_to_string(&file)?;
            let policy = GovernancePolicy::from_yaml(&content)?;
            println!(
                "OK: {} ({}), {} rule(s), scope: {}",
                policy.policy_id,
                policy.name,
                policy.rules.len(),
                if policy.agent_scope.is_empty() {
                    "all agents".to_string()
                } else {
                    policy.agent_scope.join(", ")
                }
            );
            Ok(())
        }
        PolicyCommand::ToRego { file } => {
            let content = std::fs::read_to_string(&file)?;
            let policy = GovernancePolicy::from_yaml(&content)?;
            print!("{}", policy.to_rego_stub());
            Ok(())
        }
    }
}

/// Builds a Managed Identity credential, scoped to a user-assigned
/// identity if `client_id` is given. All Azure calls in this CLI
/// authenticate this way; no client secret is ever configured.
fn credential(client_id: Option<String>) -> ManagedIdentityCredential {
    match client_id {
        Some(id) => ManagedIdentityCredential::new().with_client_id(id),
        None => ManagedIdentityCredential::new(),
    }
}

async fn run_azure_command(action: AzureCommand) -> Result<(), BoxError> {
    match action {
        AzureCommand::ListAgents { client_id } => {
            let token = credential(client_id).get_token("https://graph.microsoft.com/").await?;
            let apps = GraphClient::new().list_agent_app_registrations(&token.access_token).await?;

            if apps.is_empty() {
                println!("No app registrations tagged 'agc-agent' found.");
            } else {
                for app in apps {
                    println!(
                        "{}  {}  (object id: {})",
                        app.app_id,
                        app.display_name.as_deref().unwrap_or("(no display name)"),
                        app.id
                    );
                }
            }
            Ok(())
        }
        AzureCommand::PushAudit { file, dce_endpoint, dcr_id, stream, client_id } => {
            let content = std::fs::read_to_string(&file)?;
            let records: Vec<AuditRecord> = content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<Result<_, _>>()?;
            println!("Loaded {} audit record(s) from {}", records.len(), file.display());

            let token = credential(client_id).get_token("https://monitor.azure.com/").await?;
            MonitorIngestClient::new(dce_endpoint, dcr_id, stream)
                .push_records(&token.access_token, &records)
                .await?;
            println!("Pushed {} audit record(s) to Azure Monitor.", records.len());
            Ok(())
        }
    }
}
