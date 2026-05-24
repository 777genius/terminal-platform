mod action;
mod backend;
mod cli;
mod constants;
mod input;
mod probe;
mod rows;
mod screen;
mod session;
mod snapshot;
mod target;

#[doc(hidden)]
pub mod __fuzz;

pub use backend::ZellijBackend;

#[cfg(test)]
pub(crate) use action::ZellijAction;
#[cfg(test)]
pub(crate) use backend::capabilities_for_surface;
#[cfg(test)]
pub(crate) use constants::ZELLIJ_ROUTE_NAMESPACE;
#[cfg(test)]
pub(crate) use probe::{ZellijProbe, ZellijSurface, parse_semver_triplet};
#[cfg(test)]
pub(crate) use rows::{ZellijPaneRow, ZellijTabRow, parse_panes_json, parse_tabs_json};
#[cfg(test)]
pub(crate) use session::ZellijAttachedSession;
#[cfg(test)]
pub(crate) use session::dump_screen_scrollback_args;
#[cfg(test)]
pub(crate) use snapshot::{
    ZellijPaneKind, ZellijSessionSnapshot, build_session_snapshot, collect_pane_ids,
};
#[cfg(test)]
pub(crate) use target::ZellijTarget;

#[cfg(test)]
mod tests;
