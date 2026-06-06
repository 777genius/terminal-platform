mod launch;
mod reader;
mod spawn;

use super::model::NativePtyProcess;

pub(in crate::engine) use launch::resolve_launch_spec;
pub(in crate::engine) use spawn::{spawn_pane, spawn_tab};

impl Drop for NativePtyProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
