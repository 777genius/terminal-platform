use std::time::Duration;

pub(crate) const ZELLIJ_ROUTE_NAMESPACE: &str = "zellij_session";
pub(crate) const ZELLIJ_POLL_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const ZELLIJ_TRANSIENT_RETRY_ATTEMPTS: usize = 2;
pub(crate) const ZELLIJ_ACTION_SETTLE_ATTEMPTS: usize = 600;
pub(crate) const ZELLIJ_ACTION_SETTLE_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const ZELLIJ_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const ZELLIJ_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(25);
