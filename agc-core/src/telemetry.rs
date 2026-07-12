/// Opt-in telemetry configuration.
///
/// All telemetry is disabled by default. No data is transmitted unless
/// the operator explicitly enables it and configures an endpoint.
/// See PRIVACY.md for the full telemetry data inventory.
#[derive(Debug, Clone, Default)]
pub struct TelemetryConfig {
    /// Master opt-in switch. All other fields are ignored if `false`.
    pub enabled: bool,
    /// OTLP gRPC or HTTP endpoint (e.g. Azure Monitor OTLP endpoint).
    pub endpoint: Option<String>,
    /// Service name reported in telemetry spans.
    pub service_name: String,
    /// Include agent IDs in telemetry (may be sensitive in some deployments).
    pub include_agent_ids: bool,
}

/// No-op telemetry sink used when telemetry is disabled.
pub struct NoopTelemetry;

impl NoopTelemetry {
    pub fn record_span(&self, _operation: &str, _duration_ms: u64) {
        // intentional no-op
    }
}

/// Telemetry facade: routes to real OTLP exporter or no-op.
pub enum TelemetrySink {
    Enabled { endpoint: String, service_name: String },
    Disabled(NoopTelemetry),
}

impl TelemetrySink {
    pub fn from_config(cfg: &TelemetryConfig) -> Self {
        if cfg.enabled {
            if let Some(ep) = &cfg.endpoint {
                return Self::Enabled {
                    endpoint: ep.clone(),
                    service_name: cfg.service_name.clone(),
                };
            }
        }
        Self::Disabled(NoopTelemetry)
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    pub fn record_span(&self, operation: &str, duration_ms: u64) {
        match self {
            Self::Enabled { service_name, endpoint } => {
                tracing::debug!(service = service_name, endpoint = endpoint, op = operation, duration_ms, "telemetry span");
            }
            Self::Disabled(noop) => noop.record_span(operation, duration_ms),
        }
    }
}
