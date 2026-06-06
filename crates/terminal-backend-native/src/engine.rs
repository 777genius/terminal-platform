use std::sync::Mutex;

use terminal_domain::SessionId;
use tokio::sync::watch;

use self::model::NativeSessionState;

mod commands;
mod dispatch;
mod input;
mod layout;
mod lifecycle;
mod model;
mod process;
mod projection;
mod signals;
mod snapshots;
mod state_access;
mod subscriptions;

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;
const SNAPSHOT_HISTORY_LIMIT: usize = 64;
const SPLIT_RATIO_SCALE: u16 = 10_000;
const DEFAULT_SPLIT_RATIO_BPS: u16 = SPLIT_RATIO_SCALE / 2;

pub(crate) struct NativeSessionEngine {
    session_id: SessionId,
    state: Mutex<NativeSessionState>,
    topology_tick: watch::Sender<u64>,
}
