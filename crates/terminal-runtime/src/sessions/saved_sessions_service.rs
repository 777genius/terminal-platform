mod catalog;
mod restore;
mod save;

use super::runtime::SessionRuntime;

#[derive(Clone)]
pub(super) struct SavedSessionsService<'a> {
    runtime: SessionRuntime<'a>,
}

impl<'a> SavedSessionsService<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { runtime }
    }
}
