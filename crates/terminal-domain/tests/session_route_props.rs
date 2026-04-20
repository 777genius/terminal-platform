use proptest::prelude::*;
use terminal_domain::{
    BackendKind, ExternalSessionRef, RouteAuthority, SessionId, SessionRoute, imported_session_id,
    local_native_route, local_native_session_id,
};
use uuid::Uuid;

fn backend_kind_strategy() -> impl Strategy<Value = BackendKind> {
    prop_oneof![Just(BackendKind::Native), Just(BackendKind::Tmux), Just(BackendKind::Zellij),]
}

fn route_authority_strategy() -> impl Strategy<Value = RouteAuthority> {
    prop_oneof![Just(RouteAuthority::LocalDaemon), Just(RouteAuthority::ImportedForeign),]
}

fn session_id_strategy() -> impl Strategy<Value = SessionId> {
    any::<[u8; 16]>().prop_map(|bytes| SessionId::from(Uuid::from_bytes(bytes)))
}

fn external_ref_string_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9._:-]{0,32}"
}

proptest! {
    #![proptest_config(proptest::test_runner::Config {
        failure_persistence: None,
        .. proptest::test_runner::Config::default()
    })]

    #[test]
    fn local_native_route_roundtrips_for_any_session_id(session_id in session_id_strategy()) {
        let route = local_native_route(session_id);

        prop_assert_eq!(local_native_session_id(&route), Some(session_id));
        prop_assert_eq!(imported_session_id(&route), None);
    }

    #[test]
    fn imported_session_id_requires_foreign_authority_and_external_ref(
        backend in backend_kind_strategy(),
        namespace in external_ref_string_strategy(),
        value in external_ref_string_strategy(),
    ) {
        let local_route = SessionRoute {
            backend,
            authority: RouteAuthority::LocalDaemon,
            external: Some(ExternalSessionRef {
                namespace: namespace.clone(),
                value: value.clone(),
            }),
        };
        let missing_external_route = SessionRoute {
            backend,
            authority: RouteAuthority::ImportedForeign,
            external: None,
        };

        prop_assert_eq!(imported_session_id(&local_route), None);
        prop_assert_eq!(imported_session_id(&missing_external_route), None);
    }

    #[test]
    fn imported_session_id_changes_when_import_fingerprint_changes(
        backend_a in backend_kind_strategy(),
        backend_b in backend_kind_strategy(),
        namespace_a in external_ref_string_strategy(),
        namespace_b in external_ref_string_strategy(),
        value_a in external_ref_string_strategy(),
        value_b in external_ref_string_strategy(),
    ) {
        let route_a = SessionRoute {
            backend: backend_a,
            authority: RouteAuthority::ImportedForeign,
            external: Some(ExternalSessionRef {
                namespace: namespace_a,
                value: value_a,
            }),
        };
        let route_b = SessionRoute {
            backend: backend_b,
            authority: RouteAuthority::ImportedForeign,
            external: Some(ExternalSessionRef {
                namespace: namespace_b,
                value: value_b,
            }),
        };

        prop_assume!(route_a != route_b);

        prop_assert_ne!(
            imported_session_id(&route_a),
            imported_session_id(&route_b)
        );
    }

    #[test]
    fn local_native_session_id_only_accepts_canonical_native_routes(
        backend in backend_kind_strategy(),
        authority in route_authority_strategy(),
        namespace in external_ref_string_strategy(),
        value in external_ref_string_strategy(),
    ) {
        let route = SessionRoute {
            backend,
            authority: authority.clone(),
            external: Some(ExternalSessionRef {
                namespace: namespace.clone(),
                value: value.clone(),
            }),
        };
        let expected = if backend == BackendKind::Native
            && authority == RouteAuthority::LocalDaemon
            && namespace == "native_session"
        {
            Uuid::parse_str(&value).ok().map(SessionId::from)
        } else {
            None
        };

        prop_assert_eq!(local_native_session_id(&route), expected);
    }
}
