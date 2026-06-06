use super::super::*;

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
pub struct TerminalPersistenceV2 {
    pub(in crate::v2) path: PathBuf,
    pub(in crate::v2) config: TerminalPersistenceV2Config,
}
