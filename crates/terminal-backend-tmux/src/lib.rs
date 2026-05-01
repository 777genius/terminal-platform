#[doc(hidden)]
pub mod __fuzz;

mod backend;
mod layout;
mod prelude;
mod rows;
mod sequence;
mod session;
mod target;
mod util;

pub use backend::TmuxBackend;

pub(crate) use layout::{fallback_tree, parse_tmux_layout};
pub(crate) use target::TmuxTarget;
pub(crate) use util::tmux_split_flag;

pub(crate) const TMUX_ROUTE_NAMESPACE: &str = "tmux_target";
pub(crate) const TMUX_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

#[cfg(test)]
mod tests;
