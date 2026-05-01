use crate::verify_windows_conpty_vendor_patch;

use super::shared_data::{VALID_VENDORED_PORTABLE_PTY_PSEUDOCON, VALID_WORKSPACE_MANIFEST};

#[test]
fn verify_windows_conpty_vendor_patch_accepts_expected_contract() {
    if let Err(error) = verify_windows_conpty_vendor_patch(
        VALID_WORKSPACE_MANIFEST,
        VALID_VENDORED_PORTABLE_PTY_PSEUDOCON,
    ) {
        panic!("expected Windows ConPTY vendor patch to validate - {error}");
    }
}

#[test]
fn verify_windows_conpty_vendor_patch_rejects_missing_workspace_patch() {
    let invalid_manifest = VALID_WORKSPACE_MANIFEST
        .replace("[patch.crates-io]\nportable-pty = { path = \"vendor/portable-pty\" }\n", "");
    let error = match verify_windows_conpty_vendor_patch(
        &invalid_manifest,
        VALID_VENDORED_PORTABLE_PTY_PSEUDOCON,
    ) {
        Ok(()) => panic!("expected missing workspace patch to fail"),
        Err(error) => error,
    };

    assert!(error.contains("[patch.crates-io]"), "got: {error}");
}

#[test]
fn verify_windows_conpty_vendor_patch_rejects_undocumented_flags() {
    let invalid_psuedocon = VALID_VENDORED_PORTABLE_PTY_PSEUDOCON
        .replace("0,\n", "PSUEDOCONSOLE_INHERIT_CURSOR | PSEUDOCONSOLE_RESIZE_QUIRK,\n");
    let error =
        match verify_windows_conpty_vendor_patch(VALID_WORKSPACE_MANIFEST, &invalid_psuedocon) {
            Ok(()) => panic!("expected undocumented Windows ConPTY flags to fail"),
            Err(error) => error,
        };

    assert!(error.contains("dwFlags = 0"), "got: {error}");
}
