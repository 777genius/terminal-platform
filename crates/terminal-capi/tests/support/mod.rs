#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::{
    process::{Child, Command},
    time::Duration,
};

pub fn generate_header() -> std::io::Result<PathBuf> {
    let header_dir = unique_temp_dir("terminal-capi-header");
    let header_path = header_dir.join("terminal-platform-capi.h");
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_path = crate_dir.join("cbindgen.toml");
    let config = cbindgen::Config::from_file(&config_path).map_err(|error| {
        std::io::Error::other(format!(
            "cbindgen config should load from {} - {error}",
            config_path.display()
        ))
    })?;
    let bindings = cbindgen::Builder::new()
        .with_crate(crate_dir.display().to_string())
        .with_config(config)
        .generate()
        .map_err(|error| {
            std::io::Error::other(format!("cbindgen should generate header - {error}"))
        })?;

    if let Some(parent) = header_path.parent() {
        fs::create_dir_all(parent)?;
    }

    bindings.write_to_file(&header_path);
    Ok(header_path)
}

pub fn read_generated_header() -> std::io::Result<String> {
    let header_path = generate_header()?;
    fs::read_to_string(&header_path)
}

pub fn read_json(path: &Path) -> std::io::Result<serde_json::Value> {
    let payload = fs::read_to_string(path)?;
    serde_json::from_str(&payload).map_err(|error| {
        std::io::Error::other(format!("failed to parse json {} - {error}", path.display()))
    })
}

pub fn locate_cdylib() -> std::io::Result<PathBuf> {
    let test_binary = std::env::current_exe()?;
    let deps_dir = test_binary
        .parent()
        .ok_or_else(|| std::io::Error::other("test binary should have a parent dir"))?;
    let target_dir = deps_dir
        .parent()
        .ok_or_else(|| std::io::Error::other("deps dir should have a parent dir"))?;

    for dir in [deps_dir, target_dir] {
        for name in candidate_cdylib_names() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("could not find terminal-capi cdylib near {}", test_binary.display()),
    ))
}

#[cfg(unix)]
pub fn compile_reference_consumer(
    source_path: &Path,
    header_path: &Path,
    cdylib_path: &Path,
) -> std::io::Result<PathBuf> {
    let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let binary_path = unique_temp_path("terminal-capi-consumer", "bin");
    let lib_dir = cdylib_path
        .parent()
        .ok_or_else(|| std::io::Error::other("cdylib should have a parent dir"))?;
    let include_dir = header_path
        .parent()
        .ok_or_else(|| std::io::Error::other("header should have a parent dir"))?;
    let output = Command::new(&compiler)
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg(source_path)
        .arg("-I")
        .arg(include_dir)
        .arg("-L")
        .arg(lib_dir)
        .arg("-lterminal_capi")
        .arg(format!("-Wl,-rpath,{}", lib_dir.display()))
        .arg("-o")
        .arg(&binary_path)
        .output()
        .map_err(|error| std::io::Error::other(format!("failed to launch {compiler} - {error}")))?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "reference consumer compile failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(binary_path)
}

#[cfg(unix)]
pub fn compile_reference_consumer_with_pkg_config(
    source_path: &Path,
    package_dir: &Path,
    pkgconfig_name: &str,
) -> std::io::Result<PathBuf> {
    let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let binary_path = unique_temp_path("terminal-capi-consumer-pkgconfig", "bin");
    let pkgconfig_dir = package_dir.join("lib/pkgconfig");
    let output = Command::new("pkg-config")
        .args(["--cflags", "--libs", pkgconfig_name])
        .env("PKG_CONFIG_PATH", &pkgconfig_dir)
        .output()
        .map_err(|error| std::io::Error::other(format!("failed to launch pkg-config - {error}")))?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "pkg-config resolution failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let flags = String::from_utf8(output.stdout).map_err(|error| {
        std::io::Error::other(format!("pkg-config emitted invalid utf8 - {error}"))
    })?;
    let mut command = Command::new(&compiler);
    command.arg("-std=c11").arg("-Wall").arg("-Wextra").arg("-Werror").arg(source_path);

    for flag in flags.split_whitespace() {
        command.arg(flag);
    }

    let output =
        command.arg("-o").arg(&binary_path).output().map_err(|error| {
            std::io::Error::other(format!("failed to launch {compiler} - {error}"))
        })?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "reference consumer compile via pkg-config failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(binary_path)
}

#[cfg(unix)]
pub fn configure_runtime_library_path(command: &mut Command, cdylib_path: &Path) {
    let lib_dir = cdylib_path.parent().expect("cdylib should have a parent dir");

    #[cfg(target_os = "macos")]
    {
        command.env("DYLD_LIBRARY_PATH", lib_dir);
    }

    #[cfg(target_os = "linux")]
    {
        command.env("LD_LIBRARY_PATH", lib_dir);
    }
}

pub fn unique_temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}

pub fn unique_temp_dir(prefix: &str) -> PathBuf {
    unique_temp_path(prefix, "dir")
}

#[cfg(unix)]
pub async fn wait_for_file(path: &Path, label: &str) {
    for _ in 0..600 {
        if path.is_file() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    panic!("timed out waiting for {label} at {}", path.display());
}

#[cfg(unix)]
pub async fn wait_for_file_or_child_exit(path: &Path, mut child: Child, label: &str) -> Child {
    for _ in 0..600 {
        if path.is_file() {
            return child;
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().expect("exited child output should collect");
                panic!(
                    "{label} exited before writing {} with status {status}\nstdout:\n{}\nstderr:\n{}",
                    path.display(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Ok(None) => {}
            Err(error) => {
                panic!("failed to poll {label} while waiting for {} - {error}", path.display())
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let _ = child.kill();
    let output = child.wait_with_output().expect("timed out child output should collect");
    panic!(
        "timed out waiting for {label} at {}\nstdout:\n{}\nstderr:\n{}",
        path.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
