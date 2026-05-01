//! Thin Node/Electron leaf adapter over the safe `terminal-node` facade.

mod client;
mod json;
mod subscription;

pub use client::TerminalNodeBinding;
pub use subscription::TerminalNodeSubscriptionBinding;
