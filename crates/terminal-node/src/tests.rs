mod prelude;
mod support;

mod basic_flow;
mod export_platform;
mod restart;
mod screen;
mod subscriptions;
#[cfg(all(unix, feature = "tmux-backend"))]
mod tmux;
#[cfg(all(any(unix, windows), feature = "zellij-backend"))]
mod zellij;
