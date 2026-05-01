use crate::{prelude::*, session::TmuxAttachedSession, target::TmuxTarget};

use super::TmuxBackend;

pub(super) fn attach_tmux_session(
    backend: TmuxBackend,
    session_id: SessionId,
    route: SessionRoute,
) -> Result<Box<dyn BackendSessionPort>, BackendError> {
    if route.backend != BackendKind::Tmux {
        return Err(BackendError::invalid_input("tmux backend can only attach tmux routes"));
    }

    let target = TmuxTarget::from_route(&route)?;
    backend.run(Some(&target), &["has-session", "-t", &target.session_name])?;

    Ok(Box::new(TmuxAttachedSession { backend: Arc::new(backend), session_id, target })
        as Box<dyn BackendSessionPort>)
}
