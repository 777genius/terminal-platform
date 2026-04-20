use std::{
    env, fs,
    path::{Path, PathBuf},
};

const CAPI_PACKAGE_NAME: &str = "terminal-capi";
const CAPI_HEADER_NAME: &str = "terminal-platform-capi.h";
const CAPI_LIBRARY_BASENAME: &str = "terminal_capi";
const CAPI_SCHEMA_VERSION: u64 = 1;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match parse_command(env::args().skip(1))? {
        Command::StageCapiPackage { out_dir } => {
            let staged_dir = stage_capi_package(&out_dir)?;
            println!("{}", staged_dir.display());
            Ok(())
        }
        Command::VerifyCapiPackage { package_dir } => {
            verify_capi_package(&package_dir)?;
            println!("{}", package_dir.display());
            Ok(())
        }
    }
}

enum Command {
    StageCapiPackage { out_dir: PathBuf },
    VerifyCapiPackage { package_dir: PathBuf },
}

fn parse_command(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    let Some(command) = args.next() else {
        return Err("missing xtask command".to_string());
    };

    match command.as_str() {
        "stage-capi-package" => {
            let mut out_dir = workspace_root().join("crates/terminal-capi/artifacts/local");

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--out" => {
                        let value =
                            args.next().ok_or_else(|| "missing value for --out".to_string())?;
                        out_dir = PathBuf::from(value);
                    }
                    other => {
                        return Err(format!("unsupported stage-capi-package argument: {other}"));
                    }
                }
            }

            Ok(Command::StageCapiPackage { out_dir })
        }
        "verify-capi-package" => {
            let mut package_dir = workspace_root().join("crates/terminal-capi/artifacts/local");

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--package-dir" => {
                        let value = args
                            .next()
                            .ok_or_else(|| "missing value for --package-dir".to_string())?;
                        package_dir = PathBuf::from(value);
                    }
                    other => {
                        return Err(format!("unsupported verify-capi-package argument: {other}"));
                    }
                }
            }

            Ok(Command::VerifyCapiPackage { package_dir })
        }
        other => Err(format!("unsupported xtask command: {other}")),
    }
}

fn stage_capi_package(out_dir: &Path) -> Result<PathBuf, String> {
    let capi_dir = workspace_root().join("crates/terminal-capi");
    let include_dir = out_dir.join("include");
    let lib_dir = out_dir.join("lib");
    let header_path = include_dir.join(CAPI_HEADER_NAME);
    let package_version = read_crate_version(&capi_dir.join("Cargo.toml"))?;
    let cdylib_path = locate_artifact(candidate_cdylib_names())?;
    let staticlib_path = locate_artifact(candidate_staticlib_names())?;
    let cdylib_name = file_name(&cdylib_path)?;
    let staticlib_name = file_name(&staticlib_path)?;

    if out_dir.exists() {
        fs::remove_dir_all(out_dir)
            .map_err(|error| format!("failed to clear {} - {error}", out_dir.display()))?;
    }
    fs::create_dir_all(&include_dir)
        .map_err(|error| format!("failed to create {} - {error}", include_dir.display()))?;
    fs::create_dir_all(&lib_dir)
        .map_err(|error| format!("failed to create {} - {error}", lib_dir.display()))?;

    generate_header(&capi_dir, &header_path)?;
    copy_file(&capi_dir.join("README.md"), &out_dir.join("README.md"))?;
    copy_file(&cdylib_path, &lib_dir.join(cdylib_name))?;
    copy_file(&staticlib_path, &lib_dir.join(staticlib_name))?;

    let manifest = serde_json::json!({
        "schemaVersion": CAPI_SCHEMA_VERSION,
        "package": CAPI_PACKAGE_NAME,
        "packageVersion": package_version,
        "target": current_target_descriptor(),
        "exports": {
            "header": format!("include/{CAPI_HEADER_NAME}"),
            "cdylib": format!("lib/{cdylib_name}"),
            "staticlib": format!("lib/{staticlib_name}"),
            "libraryBaseName": CAPI_LIBRARY_BASENAME,
        }
    });

    write_json(&out_dir.join("manifest.json"), &manifest)?;
    Ok(out_dir.to_path_buf())
}

fn verify_capi_package(package_dir: &Path) -> Result<(), String> {
    let manifest_path = package_dir.join("manifest.json");
    let readme_path = package_dir.join("README.md");
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
    let header_path = package_dir.join(header_relative);
    let cdylib_path = package_dir.join(cdylib_relative);
    let staticlib_path = package_dir.join(staticlib_relative);
    let package_version = manifest
        .get("packageVersion")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest packageVersion must be a string".to_string())?;
    let target = manifest
        .get("target")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "manifest target must be an object".to_string())?;

    assert_value(
        manifest.get("schemaVersion").and_then(serde_json::Value::as_u64)
            == Some(CAPI_SCHEMA_VERSION),
        "manifest schemaVersion is unexpected",
    )?;
    assert_value(
        manifest.get("package").and_then(serde_json::Value::as_str) == Some(CAPI_PACKAGE_NAME),
        "manifest package is unexpected",
    )?;
    assert_value(!package_version.is_empty(), "manifest packageVersion is missing")?;
    assert_value(readme_path.is_file(), "staged README.md is missing")?;
    assert_value(header_path.is_file(), "staged header is missing")?;
    assert_value(cdylib_path.is_file(), "staged cdylib is missing")?;
    assert_value(staticlib_path.is_file(), "staged staticlib is missing")?;
    assert_value(
        target.get("platform").and_then(serde_json::Value::as_str) == Some(env::consts::OS),
        "manifest target.platform is unexpected",
    )?;
    assert_value(
        target.get("arch").and_then(serde_json::Value::as_str) == Some(env::consts::ARCH),
        "manifest target.arch is unexpected",
    )?;

    #[cfg(target_os = "linux")]
    assert_value(
        target.get("libc").and_then(serde_json::Value::as_str) == detect_linux_libc().as_deref(),
        "manifest target.libc is unexpected",
    )?;

    let header = fs::read_to_string(&header_path)
        .map_err(|error| format!("failed to read {} - {error}", header_path.display()))?;
    assert_value(
        header.contains("terminal_capi_client_open_subscription"),
        "staged header is missing terminal_capi_client_open_subscription",
    )?;
    assert_value(
        header.contains("terminal_capi_subscription_next_event_json"),
        "staged header is missing terminal_capi_subscription_next_event_json",
    )?;
    Ok(())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask workspace root should resolve")
        .to_path_buf()
}

fn read_crate_version(manifest_path: &Path) -> Result<String, String> {
    let manifest = fs::read_to_string(manifest_path)
        .map_err(|error| format!("failed to read {} - {error}", manifest_path.display()))?;
    manifest
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("version = \"")
                .and_then(|value| value.strip_suffix('"'))
                .map(ToOwned::to_owned)
        })
        .ok_or_else(|| format!("failed to resolve version from {}", manifest_path.display()))
}

fn generate_header(capi_dir: &Path, header_path: &Path) -> Result<(), String> {
    let config_path = capi_dir.join("cbindgen.toml");
    let config = cbindgen::Config::from_file(&config_path)
        .map_err(|error| format!("failed to load {} - {error}", config_path.display()))?;
    let bindings = cbindgen::Builder::new()
        .with_crate(capi_dir.display().to_string())
        .with_config(config)
        .generate()
        .map_err(|error| format!("failed to generate c api header - {error}"))?;
    bindings.write_to_file(header_path);
    Ok(())
}

fn locate_artifact(candidates: &[&str]) -> Result<PathBuf, String> {
    let current_exe =
        env::current_exe().map_err(|error| format!("failed to resolve current exe - {error}"))?;
    let binary_dir = current_exe
        .parent()
        .ok_or_else(|| "xtask current exe should have a parent directory".to_string())?;
    let deps_dir = binary_dir.join("deps");
    let target_dir = binary_dir
        .parent()
        .ok_or_else(|| "xtask binary dir should have a parent directory".to_string())?;

    for dir in [binary_dir, deps_dir.as_path(), target_dir] {
        for candidate in candidates {
            let path = dir.join(candidate);
            if path.is_file() {
                return Ok(path);
            }
        }
    }

    Err(format!(
        "failed to locate artifact near {} for candidates {}",
        current_exe.display(),
        candidates.join(", ")
    ))
}

fn candidate_cdylib_names() -> &'static [&'static str] {
    #[cfg(target_os = "macos")]
    {
        &["libterminal_capi.dylib"]
    }

    #[cfg(target_os = "linux")]
    {
        &["libterminal_capi.so"]
    }

    #[cfg(target_os = "windows")]
    {
        &["terminal_capi.dll"]
    }
}

fn candidate_staticlib_names() -> &'static [&'static str] {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        &["libterminal_capi.a"]
    }

    #[cfg(target_os = "windows")]
    {
        &["terminal_capi.lib", "libterminal_capi.a"]
    }
}

fn current_target_descriptor() -> serde_json::Value {
    serde_json::json!({
        "platform": env::consts::OS,
        "arch": env::consts::ARCH,
        "libc": current_libc_descriptor(),
    })
}

fn current_libc_descriptor() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        detect_linux_libc()
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_libc() -> Option<String> {
    if cfg!(target_env = "musl") { Some("musl".to_string()) } else { Some("gnu".to_string()) }
}

fn copy_file(source: &Path, target: &Path) -> Result<(), String> {
    fs::copy(source, target).map(|_| ()).map_err(|error| {
        format!("failed to copy {} to {} - {error}", source.display(), target.display())
    })
}

fn file_name(path: &Path) -> Result<&str, String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("failed to resolve file name for {}", path.display()))
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let payload = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize {} - {error}", path.display()))?;
    fs::write(path, format!("{payload}\n"))
        .map_err(|error| format!("failed to write {} - {error}", path.display()))
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let payload = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {} - {error}", path.display()))?;
    serde_json::from_str(&payload)
        .map_err(|error| format!("failed to parse {} - {error}", path.display()))
}

fn assert_value(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.to_string()) }
}
