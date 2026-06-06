mod dispatch;
mod health;
mod history_queries;
mod input_capture;
mod screen_queries;

use super::runtime::SessionRuntime;

#[derive(Clone)]
pub(super) struct ActiveSessionService<'a> {
    runtime: SessionRuntime<'a>,
}

impl<'a> ActiveSessionService<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { runtime }
    }
}
