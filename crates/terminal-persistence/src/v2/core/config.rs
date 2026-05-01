use super::super::*;

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
