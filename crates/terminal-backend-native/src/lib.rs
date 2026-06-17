mod application;
mod emulator;
mod engine;
mod subscriptions;
mod transcript;

pub use application::NativeBackend;

pub(crate) const TERMINAL_FEATURE_REPORT: &str = "T3BGsGoSyHFP";

#[cfg(test)]
mod tests;
