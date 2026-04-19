use std::{collections::HashMap, sync::RwLock};

use terminal_domain::{SessionId, SessionRoute};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDescriptor {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
}

pub trait SessionRegistry: Send + Sync {
    fn insert(&self, session: SessionDescriptor);
    fn list(&self) -> Vec<SessionDescriptor>;
}

#[derive(Debug, Default)]
pub struct InMemorySessionRegistry {
    sessions: RwLock<HashMap<SessionId, SessionDescriptor>>,
}

impl SessionRegistry for InMemorySessionRegistry {
    fn insert(&self, session: SessionDescriptor) {
        let mut sessions =
            self.sessions.write().expect("session registry write lock should not be poisoned");
        sessions.insert(session.session_id, session);
    }

    fn list(&self) -> Vec<SessionDescriptor> {
        let sessions =
            self.sessions.read().expect("session registry read lock should not be poisoned");
        sessions.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use terminal_domain::{BackendKind, RouteAuthority, SessionRoute};

    use super::{InMemorySessionRegistry, SessionDescriptor, SessionRegistry};

    #[test]
    fn inserts_and_lists_sessions() {
        let registry = InMemorySessionRegistry::default();
        let descriptor = SessionDescriptor {
            session_id: terminal_domain::SessionId::new(),
            route: SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            },
            title: Some("shell".to_string()),
        };

        registry.insert(descriptor.clone());

        let sessions = registry.list();
        assert_eq!(sessions, vec![descriptor]);
    }
}
