use super::*;

#[derive(Debug, Error)]
pub enum TerminalPersistenceV2Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("diesel connection: {0}")]
    Connection(#[from] diesel::ConnectionError),
    #[error("diesel query: {0}")]
    Query(#[from] DieselError),
    #[error("migration: {0}")]
    Migration(String),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("wrong sqlite database application_id: {application_id}")]
    WrongDatabase { application_id: i32 },
    #[error("wrong terminal database identity: product={product}, schema_family={schema_family}")]
    IdentityMismatch { product: String, schema_family: String },
    #[error("invalid terminal persistence data: {0}")]
    InvalidData(String),
    #[error("an active terminal writer generation already exists")]
    WriterAlreadyActive,
    #[error("terminal persistence executor stopped")]
    ExecutorStopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurabilityProfile {
    ReliableHistory,
    PerformanceHistory,
    Test,
}

impl DurabilityProfile {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReliableHistory => "reliable_history",
            Self::PerformanceHistory => "performance_history",
            Self::Test => "test",
        }
    }

    #[must_use]
    pub fn sqlite_synchronous(self) -> &'static str {
        match self {
            Self::ReliableHistory => "FULL",
            Self::PerformanceHistory | Self::Test => "NORMAL",
        }
    }
}

impl Default for DurabilityProfile {
    fn default() -> Self {
        Self::ReliableHistory
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureGateName {
    TerminalPersistenceV2Shadow,
    TerminalPersistenceV2Capture,
    TerminalPersistenceV2AuthoritativeReads,
    TerminalPersistenceV2Authoritative,
    MuxStructuredCapture,
    SegmentCompressionZstd,
    RawHistoryExport,
    EncryptedTerminalHistory,
}

impl FeatureGateName {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TerminalPersistenceV2Shadow => "terminal_persistence_v2_shadow",
            Self::TerminalPersistenceV2Capture => "terminal_persistence_v2_capture",
            Self::TerminalPersistenceV2AuthoritativeReads => {
                "terminal_persistence_v2_authoritative_reads"
            }
            Self::TerminalPersistenceV2Authoritative => "terminal_persistence_v2_authoritative",
            Self::MuxStructuredCapture => "mux_structured_capture",
            Self::SegmentCompressionZstd => "segment_compression_zstd",
            Self::RawHistoryExport => "raw_history_export",
            Self::EncryptedTerminalHistory => "encrypted_terminal_history",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureGateState {
    Disabled,
    Shadow,
    Enabled,
    ForceDisabled,
}

impl FeatureGateState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Shadow => "shadow",
            Self::Enabled => "enabled",
            Self::ForceDisabled => "force_disabled",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self, TerminalPersistenceV2Error> {
        match value {
            "disabled" => Ok(Self::Disabled),
            "shadow" => Ok(Self::Shadow),
            "enabled" => Ok(Self::Enabled),
            "force_disabled" => Ok(Self::ForceDisabled),
            other => Err(TerminalPersistenceV2Error::InvalidData(format!(
                "unknown feature gate state: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PersistenceClock;

impl PersistenceClock {
    #[must_use]
    pub fn now_ms(self) -> i64 {
        current_time_ms()
    }
}

impl Default for PersistenceClock {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct TerminalPersistenceV2Config {
    pub durability_profile: DurabilityProfile,
    pub busy_timeout_ms: u32,
    pub wal_autocheckpoint_pages: u32,
    pub clock: PersistenceClock,
    pub storage_pressure: StoragePressureConfig,
    pub failpoints: PersistenceFailpoints,
    pub allow_test_plaintext_crypto_keys: bool,
}

impl TerminalPersistenceV2Config {
    #[must_use]
    pub fn reliable_history() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn test() -> Self {
        Self {
            durability_profile: DurabilityProfile::Test,
            busy_timeout_ms: 1_000,
            wal_autocheckpoint_pages: 64,
            clock: PersistenceClock,
            storage_pressure: StoragePressureConfig::default(),
            failpoints: PersistenceFailpoints::default(),
            allow_test_plaintext_crypto_keys: true,
        }
    }
}

impl Default for TerminalPersistenceV2Config {
    fn default() -> Self {
        Self {
            durability_profile: DurabilityProfile::ReliableHistory,
            busy_timeout_ms: 5_000,
            wal_autocheckpoint_pages: 1_000,
            clock: PersistenceClock,
            storage_pressure: StoragePressureConfig::default(),
            failpoints: PersistenceFailpoints::default(),
            allow_test_plaintext_crypto_keys: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoragePressureConfig {
    pub db_warning_bytes: i64,
    pub wal_warning_bytes: i64,
}

impl Default for StoragePressureConfig {
    fn default() -> Self {
        Self {
            db_warning_bytes: DEFAULT_DB_PRESSURE_WARNING_BYTES,
            wal_warning_bytes: DEFAULT_WAL_PRESSURE_WARNING_BYTES,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PersistenceFailpoints {
    pub stream_segment_after_segment_insert: bool,
    pub stream_segment_before_transaction_storage_full: bool,
}

#[derive(Debug, Clone)]
pub struct TerminalPersistenceV2 {
    pub(super) path: PathBuf,
    pub(super) config: TerminalPersistenceV2Config,
}
