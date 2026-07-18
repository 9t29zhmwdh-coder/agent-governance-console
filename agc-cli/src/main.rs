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
