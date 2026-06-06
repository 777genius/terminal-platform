use crate::verify_v1_release_configs;

use super::shared_data::{VALID_DENY_CONFIG, VALID_RELEASE_PLZ_CONFIG};

#[test]
fn verify_v1_release_configs_accepts_expected_governance() {
    if let Err(error) = verify_v1_release_configs(VALID_RELEASE_PLZ_CONFIG, VALID_DENY_CONFIG) {
        panic!("expected release configs to validate - {error}");
    }
}

#[test]
fn verify_v1_release_configs_rejects_dirty_release_plz() {
    let invalid_release_plz =
        VALID_RELEASE_PLZ_CONFIG.replace("allow_dirty = false", "allow_dirty = true");
    let error = match verify_v1_release_configs(&invalid_release_plz, VALID_DENY_CONFIG) {
        Ok(()) => panic!("expected dirty release-plz config to fail"),
        Err(error) => error,
    };

    assert!(error.contains("allow_dirty = false"), "got: {error}");
}

#[test]
fn verify_v1_release_configs_rejects_weak_deny_sources() {
    let invalid_deny =
        VALID_DENY_CONFIG.replace("unknown-git = \"deny\"", "unknown-git = \"warn\"");
    let error = match verify_v1_release_configs(VALID_RELEASE_PLZ_CONFIG, &invalid_deny) {
        Ok(()) => panic!("expected weak cargo-deny config to fail"),
        Err(error) => error,
    };

    assert!(error.contains("unknown-git = \"deny\""), "got: {error}");
}
