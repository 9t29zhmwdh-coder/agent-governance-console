//! Agent Governance Console: Azure integration.
//!
//! All four pieces are opt-in and independent of each other:
//! - [`ManagedIdentityCredential`]: AAD tokens via IMDS, no client secret.
//! - [`MonitorIngestClient`]: push audit records to a Log Analytics DCR.
//! - [`GraphClient`]: read agent app registrations tagged `agc-agent`.
//! - [`OtlpExporter`]: real OTLP span export.
//!
//! See `docs/azure_integration.md` for the required Azure-side setup
//! (`scripts/azure_setup.sh` provisions it) and note in that doc which
//! parts are unit/mock-tested here vs. verified against a real Azure
//! subscription (managed identity's IMDS endpoint is only reachable from
//! inside Azure, so it cannot be integration-tested from anywhere else).

mod error;
mod graph;
mod managed_identity;
mod monitor_ingest;
mod otlp;

pub use error::AzureError;
pub use graph::{AppRegistration, GraphClient};
pub use managed_identity::{AccessToken, ManagedIdentityCredential, DEFAULT_IMDS_ENDPOINT};
pub use monitor_ingest::MonitorIngestClient;
pub use otlp::OtlpExporter;
