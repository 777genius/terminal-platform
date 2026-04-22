use std::{collections::HashMap, sync::RwLock};

use terminal_backend_api::ShellLaunchSpec;
use terminal_domain::{SessionId, SessionRoute};
use terminal_projection::SessionHealthSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDescriptor {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub health: SessionHealthSnapshot,
}

pub trait SessionRegistry: Send + Sync {
    fn insert(&self, session: SessionDescriptor);
    fn get(&self, session_id: SessionId) -> Option<SessionDescriptor>;
    fn get_by_route(&self, route: &SessionRoute) -> Option<SessionDescriptor>;
    fn list(&self) -> Vec<SessionDescriptor>;
    fn update_title(&self, session_id: SessionId, title: Option<String>);
    fn update_health(&self, session_id: SessionId, health: SessionHealthSnapshot);
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

    fn get(&self, session_id: SessionId) -> Option<SessionDescriptor> {
        let sessions =
            self.sessions.read().expect("session registry read lock should not be poisoned");
        sessions.get(&session_id).cloned()
    }

    fn get_by_route(&self, route: &SessionRoute) -> Option<SessionDescriptor> {
        let sessions =
            self.sessions.read().expect("session registry read lock should not be poisoned");
        sessions.values().find(|session| &session.route == route).cloned()
    }

    fn list(&self) -> Vec<SessionDescriptor> {
        let sessions =
            self.sessions.read().expect("session registry read lock should not be poisoned");
        sessions.values().cloned().collect()
    }

    fn update_title(&self, session_id: SessionId, title: Option<String>) {
        let mut sessions =
            self.sessions.write().expect("session registry write lock should not be poisoned");
        if let Some(session) = sessions.get_mut(&session_id) {
            session.title = title;
        }
    }

    fn update_health(&self, session_id: SessionId, health: SessionHealthSnapshot) {
        let mut sessions =
            self.sessions.write().expect("session registry write lock should not be poisoned");
        if let Some(session) = sessions.get_mut(&session_id) {
            session.health = health;
        }
    }
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::ShellLaunchSpec;
    use terminal_domain::{BackendKind, RouteAuthority, SessionRoute};
    use terminal_projection::{SessionHealthPhase, SessionHealthSnapshot};

    use super::{InMemorySessionRegistry, SessionDescriptor, SessionRegistry};

    #[test]
    fn inserts_and_lists_sessions() {
        let registry = InMemorySessionRegistry::default();
        let session_id = terminal_domain::SessionId::new();
        let descriptor = SessionDescriptor {
            session_id,
            route: SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            },
            title: Some("shell".to_string()),
            launch: Some(ShellLaunchSpec::new("/bin/sh")),
            health: SessionHealthSnapshot::ready(session_id),
        };

        registry.insert(descriptor.clone());

        assert_eq!(registry.get(descriptor.session_id), Some(descriptor.clone()));
        assert_eq!(registry.get_by_route(&descriptor.route), Some(descriptor.clone()));

        let sessions = registry.list();
        assert_eq!(sessions, vec![descriptor]);
    }

    #[test]
    fn updates_existing_session_title() {
        let registry = InMemorySessionRegistry::default();
        let session_id = terminal_domain::SessionId::new();
        let descriptor = SessionDescriptor {
            session_id,
            route: SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            },
            title: Some("shell".to_string()),
            launch: Some(ShellLaunchSpec::new("/bin/sh")),
            health: SessionHealthSnapshot::ready(session_id),
        };

        registry.insert(descriptor.clone());
        registry.update_title(descriptor.session_id, Some("logs".to_string()));

        assert_eq!(
            registry.get(descriptor.session_id).and_then(|session| session.title),
            Some("logs".to_string())
        );
    }

    #[test]
    fn updates_existing_session_health() {
        let registry = InMemorySessionRegistry::default();
        let session_id = terminal_domain::SessionId::new();
        let descriptor = SessionDescriptor {
            session_id,
            route: SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            },
            title: Some("shell".to_string()),
            launch: Some(ShellLaunchSpec::new("/bin/sh")),
            health: SessionHealthSnapshot::ready(session_id),
        };

        registry.insert(descriptor);
        registry.update_health(
            session_id,
            SessionHealthSnapshot::stale(
                session_id,
                terminal_projection::SessionHealthReason::SubscriptionSourceClosed,
                "stream ended",
            ),
        );

        assert_eq!(
            registry.get(session_id).map(|session| session.health.phase),
            Some(SessionHealthPhase::Stale)
        );
    }
}
