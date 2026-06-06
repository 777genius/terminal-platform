use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    constants::*,
    support::{assert_value, copy_file_ensuring_parent, read_json, write_json},
};

use super::package::current_target_descriptor;

pub(crate) fn install_capi_package(package_dir: &Path, prefix: &Path) -> Result<PathBuf, String> {
    let manifest_path = package_dir.join("manifest.json");
    let manifest = read_json(&manifest_path)?;
    let exports =
        manifest.get("exports").and_then(serde_json::Value::as_object).ok_or_else(|| {
            format!("manifest is missing exports object at {}", manifest_path.display())
        })?;
    let header_relative = exports
        .get("header")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest exports.header must be a string".to_string())?;
    let cdylib_relative = exports
        .get("cdylib")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest exports.cdylib must be a string".to_string())?;
    let staticlib_relative = exports
        .get("staticlib")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest exports.staticlib must be a string".to_string())?;
    let pkgconfig_relative = exports
        .get("pkgConfig")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest exports.pkgConfig must be a string".to_string())?;
    let package_version = manifest
        .get("packageVersion")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest packageVersion must be a string".to_string())?;

    if prefix.exists() {
        fs::remove_dir_all(prefix)
            .map_err(|error| format!("failed to clear {} - {error}", prefix.display()))?;
    }

    copy_file_ensuring_parent(&package_dir.join(header_relative), &prefix.join(header_relative))?;
    copy_file_ensuring_parent(&package_dir.join(cdylib_relative), &prefix.join(cdylib_relative))?;
    copy_file_ensuring_parent(
        &package_dir.join(staticlib_relative),
        &prefix.join(staticlib_relative),
    )?;
    copy_file_ensuring_parent(
        &package_dir.join(pkgconfig_relative),
        &prefix.join(pkgconfig_relative),
    )?;

    let installed_readme = prefix.join(CAPI_INSTALL_SHARE_DIR).join("README.md");
    let installed_manifest_path = prefix.join(CAPI_INSTALL_SHARE_DIR).join("manifest.json");
    copy_file_ensuring_parent(&package_dir.join("README.md"), &installed_readme)?;

    let installed_manifest = serde_json::json!({
        "schemaVersion": CAPI_SCHEMA_VERSION,
        "package": CAPI_PACKAGE_NAME,
        "packageVersion": package_version,
        "layout": "prefix",
        "target": manifest
            .get("target")
            .cloned()
            .unwrap_or_else(current_target_descriptor),
        "exports": {
            "header": header_relative,
            "cdylib": cdylib_relative,
            "staticlib": staticlib_relative,
            "pkgConfig": pkgconfig_relative,
            "libraryBaseName": CAPI_LIBRARY_BASENAME,
            "metadata": format!("{CAPI_INSTALL_SHARE_DIR}/manifest.json"),
            "readme": format!("{CAPI_INSTALL_SHARE_DIR}/README.md"),
        }
    });
    write_json(&installed_manifest_path, &installed_manifest)?;
    Ok(prefix.to_path_buf())
}

pub(crate) fn verify_capi_install(prefix: &Path) -> Result<(), String> {
    let installed_manifest_path = prefix.join(CAPI_INSTALL_SHARE_DIR).join("manifest.json");
    let manifest = read_json(&installed_manifest_path)?;
    let exports =
        manifest.get("exports").and_then(serde_json::Value::as_object).ok_or_else(|| {
            format!(
                "installed manifest is missing exports object at {}",
                installed_manifest_path.display()
            )
        })?;
    let header_path = prefix.join(
        exports["header"]
            .as_str()
            .ok_or_else(|| "installed exports.header must be a string".to_string())?,
    );
    let cdylib_path = prefix.join(
        exports["cdylib"]
            .as_str()
            .ok_or_else(|| "installed exports.cdylib must be a string".to_string())?,
    );
    let staticlib_path = prefix.join(
        exports["staticlib"]
            .as_str()
            .ok_or_else(|| "installed exports.staticlib must be a string".to_string())?,
    );
    let pkgconfig_path = prefix.join(
        exports["pkgConfig"]
            .as_str()
            .ok_or_else(|| "installed exports.pkgConfig must be a string".to_string())?,
    );
    let readme_path = prefix.join(
        exports["readme"]
            .as_str()
            .ok_or_else(|| "installed exports.readme must be a string".to_string())?,
    );
    let package_version = manifest
        .get("packageVersion")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "installed manifest packageVersion must be a string".to_string())?;

    assert_value(
        manifest.get("layout").and_then(serde_json::Value::as_str) == Some("prefix"),
        "installed manifest layout is unexpected",
    )?;
    assert_value(header_path.is_file(), "installed header is missing")?;
    assert_value(cdylib_path.is_file(), "installed cdylib is missing")?;
    assert_value(staticlib_path.is_file(), "installed staticlib is missing")?;
    assert_value(pkgconfig_path.is_file(), "installed pkg-config file is missing")?;
    assert_value(readme_path.is_file(), "installed README is missing")?;

    let pkgconfig = fs::read_to_string(&pkgconfig_path)
        .map_err(|error| format!("failed to read {} - {error}", pkgconfig_path.display()))?;
    assert_value(
        pkgconfig.contains(&format!("Version: {package_version}")),
        "installed pkg-config file is missing expected package version",
    )?;
    assert_value(
        pkgconfig.contains("prefix=${pcfiledir}/../.."),
        "installed pkg-config file is missing relative prefix",
    )?;
    Ok(())
}
