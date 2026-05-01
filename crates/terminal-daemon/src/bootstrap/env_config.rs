use std::env;

use crate::bootstrap::{
    config::TerminalDaemonBootstrapConfig,
    errors::TerminalDaemonBootstrapConfigError,
    parser::{normalize_backends, parse_backend_kind},
};

impl TerminalDaemonBootstrapConfig {
    pub fn from_env() -> Result<Self, TerminalDaemonBootstrapConfigError> {
        match env::var(Self::BACKENDS_ENV) {
            Ok(value) => Self::from_backend_csv(&value),
            Err(env::VarError::NotPresent) => Ok(Self::default()),
            Err(env::VarError::NotUnicode(_)) => {
                Err(TerminalDaemonBootstrapConfigError::InvalidEnvironmentEncoding {
                    env_var: Self::BACKENDS_ENV,
                })
            }
        }
    }

    pub fn from_backend_csv(value: &str) -> Result<Self, TerminalDaemonBootstrapConfigError> {
        let enabled_backends = value
            .split(',')
            .map(str::trim)
            .filter(|candidate| !candidate.is_empty())
            .map(parse_backend_kind)
            .collect::<Result<Vec<_>, _>>()?;

        if enabled_backends.is_empty() {
            return Err(TerminalDaemonBootstrapConfigError::EmptyBackendList);
        }

        Ok(Self { enabled_backends: normalize_backends(enabled_backends) })
    }
}
