use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

const CAPI_PACKAGE_NAME: &str = "terminal-capi";
const CAPI_HEADER_NAME: &str = "terminal-platform-capi.h";
const CAPI_LIBRARY_BASENAME: &str = "terminal_capi";
const CAPI_PKGCONFIG_NAME: &str = "terminal-platform-capi";
const CAPI_INSTALL_SHARE_DIR: &str = "share/terminal-capi";
const CAPI_SCHEMA_VERSION: u64 = 1;
const LICENSE_PATH: &str = "LICENSE";
const CONTRIBUTING_PATH: &str = "CONTRIBUTING.md";
const SECURITY_PATH: &str = "SECURITY.md";
const CODE_OF_CONDUCT_PATH: &str = "CODE_OF_CONDUCT.md";
const ROOT_README_PATH: &str = "README.md";
const NODE_PACKAGE_README_PATH: &str = "crates/terminal-node-napi/package/README.md";
const MANUAL_DIR: &str = "crates/terminal-testing/manual";
const MANUAL_DRAFTS_DIR: &str = "crates/terminal-testing/manual/drafts";
const MANUAL_RUNS_DIR: &str = "crates/terminal-testing/manual/runs";
const RELEASE_READINESS_WORKFLOW_PATH: &str = ".github/workflows/release-readiness.yml";
const RELEASE_CANDIDATE_SUMMARY_PATH: &str = "docs/terminal/v1-release-candidate-summary.md";
const RELEASE_SUMMARY_TEMPLATE_PATH: &str = "docs/terminal/v1-release-summary-template.md";
const MANUAL_RUN_TEMPLATE_DATE_PLACEHOLDER: &str = "Date: YYYY-MM-DD";
const MANUAL_RUN_TEMPLATE_OS_PLACEHOLDER: &str = "OS: macOS 15.4 / Ubuntu 24.04 / Windows 11 24H2";
const MANUAL_RUN_TEMPLATE_CHECKLIST_PLACEHOLDER: &str =
    "Checklist: crates/terminal-testing/manual/<checklist>.md";
const MANUAL_RUN_TEMPLATE_RUST_PLACEHOLDER: &str = "Rust: rustc 1.xx.x";
const MANUAL_RUN_TEMPLATE_NODE_PLACEHOLDER: &str = "Node: vxx.x.x";

#[derive(Clone, Copy)]
struct ManualRunExpectation {
    file_prefix: &'static str,
    checklist_path: &'static str,
    required_runtime_marker: Option<(&'static str, &'static str)>,
}

const REQUIRED_MANUAL_RUNS: [ManualRunExpectation; 3] = [
    ManualRunExpectation {
        file_prefix: "electron-",
        checklist_path: "crates/terminal-testing/manual/electron.md",
        required_runtime_marker: None,
    },
    ManualRunExpectation {
        file_prefix: "unix-tmux-",
        checklist_path: "crates/terminal-testing/manual/tmux.md",
        required_runtime_marker: Some(("tmux:", "tmux: 3.x or n/a")),
    },
    ManualRunExpectation {
        file_prefix: "windows-native-zellij-",
        checklist_path: "crates/terminal-testing/manual/windows-native-zellij.md",
        required_runtime_marker: Some(("Zellij:", "Zellij: 0.44.x or n/a")),
    },
];

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match parse_command(env::args().skip(1))? {
        Command::ExportSdkRuntimeTypes { out_dir } => {
            let exported_dir = export_sdk_runtime_types(&out_dir)?;
            println!("{}", exported_dir.display());
            Ok(())
        }
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
        Command::InstallCapiPackage { package_dir, prefix } => {
            let installed_prefix = install_capi_package(&package_dir, &prefix)?;
            println!("{}", installed_prefix.display());
            Ok(())
        }
        Command::VerifyCapiInstall { prefix } => {
            verify_capi_install(&prefix)?;
            println!("{}", prefix.display());
            Ok(())
        }
        Command::VerifyV1Readiness { require_recorded_passes } => {
            verify_v1_readiness(require_recorded_passes)?;
            println!("v1 readiness audit passed");
            Ok(())
        }
        Command::ScaffoldManualRun { kind, date, output, os, rust, node, tmux, zellij, force } => {
            let output_path = scaffold_manual_run(
                kind,
                &date,
                ManualRunScaffoldOptions { output, os, rust, node, tmux, zellij, force },
            )?;
            println!("{}", output_path.display());
            Ok(())
        }
    }
}

enum Command {
    ExportSdkRuntimeTypes {
        out_dir: PathBuf,
    },
    StageCapiPackage {
        out_dir: PathBuf,
    },
    VerifyCapiPackage {
        package_dir: PathBuf,
    },
    InstallCapiPackage {
        package_dir: PathBuf,
        prefix: PathBuf,
    },
    VerifyCapiInstall {
        prefix: PathBuf,
    },
    VerifyV1Readiness {
        require_recorded_passes: bool,
    },
    ScaffoldManualRun {
        kind: ManualRunKind,
        date: String,
        output: Option<PathBuf>,
        os: Option<String>,
        rust: Option<String>,
        node: Option<String>,
        tmux: Option<String>,
        zellij: Option<String>,
        force: bool,
    },
}

#[derive(Clone, Copy)]
enum ManualRunKind {
    Electron,
    UnixTmux,
    WindowsNativeZellij,
}

struct ManualRunScaffoldOptions {
    output: Option<PathBuf>,
    os: Option<String>,
    rust: Option<String>,
    node: Option<String>,
    tmux: Option<String>,
    zellij: Option<String>,
    force: bool,
}

fn parse_command(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    let Some(command) = args.next() else {
        return Err("missing xtask command".to_string());
    };

    match command.as_str() {
        "export-sdk-runtime-types" => {
            let mut out_dir = workspace_root().join("sdk/packages/runtime-types/src/generated/raw");

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--out" => {
                        let value =
                            args.next().ok_or_else(|| "missing value for --out".to_string())?;
                        let candidate = PathBuf::from(value);
                        out_dir = if candidate.is_absolute() {
                            candidate
                        } else {
                            workspace_root().join(candidate)
                        };
                    }
                    other => {
                        return Err(format!(
                            "unsupported export-sdk-runtime-types argument: {other}"
                        ));
                    }
                }
            }

            Ok(Command::ExportSdkRuntimeTypes { out_dir })
        }
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
        "install-capi-package" => {
            let mut package_dir = workspace_root().join("crates/terminal-capi/artifacts/local");
            let mut prefix = workspace_root().join("crates/terminal-capi/artifacts/install");

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--package-dir" => {
                        let value = args
                            .next()
                            .ok_or_else(|| "missing value for --package-dir".to_string())?;
                        package_dir = PathBuf::from(value);
                    }
                    "--prefix" => {
                        let value =
                            args.next().ok_or_else(|| "missing value for --prefix".to_string())?;
                        prefix = PathBuf::from(value);
                    }
                    other => {
                        return Err(format!("unsupported install-capi-package argument: {other}"));
                    }
                }
            }

            Ok(Command::InstallCapiPackage { package_dir, prefix })
        }
        "verify-capi-install" => {
            let mut prefix = workspace_root().join("crates/terminal-capi/artifacts/install");

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--prefix" => {
                        let value =
                            args.next().ok_or_else(|| "missing value for --prefix".to_string())?;
                        prefix = PathBuf::from(value);
                    }
                    other => {
                        return Err(format!("unsupported verify-capi-install argument: {other}"));
                    }
                }
            }

            Ok(Command::VerifyCapiInstall { prefix })
        }
        "verify-v1-readiness" => {
            let mut require_recorded_passes = false;

            for arg in args {
                match arg.as_str() {
                    "--require-recorded-passes" => {
                        require_recorded_passes = true;
                    }
                    other => {
                        return Err(format!("unsupported verify-v1-readiness argument: {other}"));
                    }
                }
            }

            Ok(Command::VerifyV1Readiness { require_recorded_passes })
        }
        "scaffold-manual-run" => {
            let mut kind = None;
            let mut date = None;
            let mut output = None;
            let mut os = None;
            let mut rust = None;
            let mut node = None;
            let mut tmux = None;
            let mut zellij = None;
            let mut force = false;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--kind" => {
                        let value =
                            args.next().ok_or_else(|| "missing value for --kind".to_string())?;
                        kind = Some(parse_manual_run_kind(&value)?);
                    }
                    "--date" => {
                        let value =
                            args.next().ok_or_else(|| "missing value for --date".to_string())?;
                        date = Some(value);
                    }
                    "--out" => {
                        let value =
                            args.next().ok_or_else(|| "missing value for --out".to_string())?;
                        output = Some(PathBuf::from(value));
                    }
                    "--os" => {
                        os = Some(args.next().ok_or_else(|| "missing value for --os".to_string())?);
                    }
                    "--rust" => {
                        rust = Some(
                            args.next().ok_or_else(|| "missing value for --rust".to_string())?,
                        );
                    }
                    "--node" => {
                        node = Some(
                            args.next().ok_or_else(|| "missing value for --node".to_string())?,
                        );
                    }
                    "--tmux" => {
                        tmux = Some(
                            args.next().ok_or_else(|| "missing value for --tmux".to_string())?,
                        );
                    }
                    "--zellij" => {
                        zellij = Some(
                            args.next().ok_or_else(|| "missing value for --zellij".to_string())?,
                        );
                    }
                    "--force" => {
                        force = true;
                    }
                    other => {
                        return Err(format!("unsupported scaffold-manual-run argument: {other}"));
                    }
                }
            }

            let kind = kind.ok_or_else(|| "missing required --kind".to_string())?;
            let date = date.ok_or_else(|| "missing required --date".to_string())?;

            Ok(Command::ScaffoldManualRun {
                kind,
                date,
                output,
                os,
                rust,
                node,
                tmux,
                zellij,
                force,
            })
        }
        other => Err(format!("unsupported xtask command: {other}")),
    }
}

fn verify_v1_readiness(require_recorded_passes: bool) -> Result<(), String> {
    let workspace_root = workspace_root();
    let license = workspace_root.join(LICENSE_PATH);
    let contributing = workspace_root.join(CONTRIBUTING_PATH);
    let security = workspace_root.join(SECURITY_PATH);
    let code_of_conduct = workspace_root.join(CODE_OF_CONDUCT_PATH);
    let root_readme = workspace_root.join(ROOT_README_PATH);
    let node_package_readme = workspace_root.join(NODE_PACKAGE_README_PATH);
    let manual_dir = workspace_root.join(MANUAL_DIR);
    let manual_drafts_dir = workspace_root.join(MANUAL_DRAFTS_DIR);
    let manual_runs_dir = workspace_root.join(MANUAL_RUNS_DIR);
    let release_readiness_workflow = workspace_root.join(RELEASE_READINESS_WORKFLOW_PATH);
    let release_candidate_summary = workspace_root.join(RELEASE_CANDIDATE_SUMMARY_PATH);
    let release_summary_template = workspace_root.join(RELEASE_SUMMARY_TEMPLATE_PATH);

    assert_value(license.is_file(), "root LICENSE is missing")?;
    assert_value(contributing.is_file(), "root CONTRIBUTING.md is missing")?;
    assert_value(security.is_file(), "root SECURITY.md is missing")?;
    assert_value(code_of_conduct.is_file(), "root CODE_OF_CONDUCT.md is missing")?;
    assert_value(root_readme.is_file(), "root README is missing")?;
    assert_value(node_package_readme.is_file(), "Node package README is missing")?;
    assert_value(manual_dir.is_dir(), "manual QA directory is missing")?;
    assert_value(manual_drafts_dir.is_dir(), "manual draft capture directory is missing")?;
    assert_value(manual_runs_dir.is_dir(), "manual run capture directory is missing")?;
    assert_value(release_readiness_workflow.is_file(), "release readiness workflow is missing")?;
    assert_value(release_candidate_summary.is_file(), "release candidate summary is missing")?;
    assert_value(release_summary_template.is_file(), "release summary template is missing")?;

    let root_readme_contents = fs::read_to_string(&root_readme)
        .map_err(|error| format!("failed to read {} - {error}", root_readme.display()))?;
    let node_package_readme_contents = fs::read_to_string(&node_package_readme)
        .map_err(|error| format!("failed to read {} - {error}", node_package_readme.display()))?;
    let release_candidate_summary_contents = fs::read_to_string(&release_candidate_summary)
        .map_err(|error| {
            format!("failed to read {} - {error}", release_candidate_summary.display())
        })?;

    for expected_line in [
        "- `macOS + Linux` - `Native + tmux + Zellij`",
        "- `Windows` - `Native + Zellij`",
        "- `tmux` stays Unix-only in v1 docs, tests, CI, and acceptance",
    ] {
        assert_value(
            root_readme_contents.contains(expected_line),
            &format!("root README is missing support matrix line: {expected_line}"),
        )?;
    }

    for expected_line in [
        "- `macOS + Linux` - `Native + tmux + Zellij`",
        "- `Windows` - `Native + Zellij`",
        "- `tmux` stays Unix-only in v1 acceptance and docs",
    ] {
        assert_value(
            node_package_readme_contents.contains(expected_line),
            &format!("Node package README is missing support matrix line: {expected_line}"),
        )?;
    }

    for expected_line in [
        "- `macOS + Linux` - `Native + tmux + Zellij`",
        "- `Windows` - `Native + Zellij`",
        "- `tmux` remains Unix-only in docs, CI, and acceptance",
    ] {
        assert_value(
            release_candidate_summary_contents.contains(expected_line),
            &format!("release candidate summary is missing support matrix line: {expected_line}"),
        )?;
    }

    assert_value(
        !release_candidate_summary_contents.contains("TODO"),
        "release candidate summary still contains TODO placeholders",
    )?;
    assert_value(
        !release_candidate_summary_contents.contains("TBD"),
        "release candidate summary still contains TBD placeholders",
    )?;

    for relative_path in [
        "README.md",
        "electron.md",
        "native.md",
        "tmux.md",
        "windows-native-zellij.md",
        "zellij.md",
    ] {
        let path = manual_dir.join(relative_path);
        assert_value(path.is_file(), &format!("manual checklist is missing: {}", path.display()))?;
    }

    let drafts_readme = manual_drafts_dir.join("README.md");
    assert_value(
        drafts_readme.is_file(),
        &format!("manual draft helper is missing: {}", drafts_readme.display()),
    )?;

    for relative_path in ["README.md", "_template.md"] {
        let path = manual_runs_dir.join(relative_path);
        assert_value(
            path.is_file(),
            &format!("manual run artifact helper is missing: {}", path.display()),
        )?;
    }

    if require_recorded_passes {
        verify_recorded_passes(&manual_runs_dir)?;
    }

    Ok(())
}

fn verify_recorded_passes(manual_runs_dir: &Path) -> Result<(), String> {
    let mut has_electron_pass = false;
    let mut has_tmux_pass = false;
    let mut has_windows_zellij_pass = false;

    for entry in fs::read_dir(manual_runs_dir)
        .map_err(|error| format!("failed to read {} - {error}", manual_runs_dir.display()))?
    {
        let entry = entry
            .map_err(|error| format!("failed to read manual run directory entry - {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if matches!(name, "README.md" | "_template.md") {
            continue;
        }

        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {} - {error}", path.display()))?;
        let Some(expectation) = REQUIRED_MANUAL_RUNS
            .iter()
            .find(|expectation| name.starts_with(expectation.file_prefix))
        else {
            continue;
        };

        verify_recorded_pass(&path, name, &contents, *expectation)?;

        match expectation.file_prefix {
            "electron-" => has_electron_pass = true,
            "unix-tmux-" => has_tmux_pass = true,
            "windows-native-zellij-" => has_windows_zellij_pass = true,
            _ => {}
        }
    }

    assert_value(has_electron_pass, "missing recorded Electron embed pass in manual/runs")?;
    assert_value(has_tmux_pass, "missing recorded Unix tmux pass in manual/runs")?;
    assert_value(
        has_windows_zellij_pass,
        "missing recorded Windows Native + Zellij pass in manual/runs",
    )?;

    Ok(())
}

fn parse_manual_run_kind(value: &str) -> Result<ManualRunKind, String> {
    match value {
        "electron" => Ok(ManualRunKind::Electron),
        "unix-tmux" => Ok(ManualRunKind::UnixTmux),
        "windows-native-zellij" => Ok(ManualRunKind::WindowsNativeZellij),
        other => Err(format!("unsupported manual run kind: {other}")),
    }
}

impl ManualRunKind {
    fn file_prefix(self) -> &'static str {
        match self {
            Self::Electron => "electron-",
            Self::UnixTmux => "unix-tmux-",
            Self::WindowsNativeZellij => "windows-native-zellij-",
        }
    }

    fn checklist_path(self) -> &'static str {
        match self {
            Self::Electron => "crates/terminal-testing/manual/electron.md",
            Self::UnixTmux => "crates/terminal-testing/manual/tmux.md",
            Self::WindowsNativeZellij => "crates/terminal-testing/manual/windows-native-zellij.md",
        }
    }

    fn default_tmux_value(self) -> &'static str {
        match self {
            Self::UnixTmux => "",
            Self::Electron | Self::WindowsNativeZellij => "n/a",
        }
    }

    fn default_zellij_value(self) -> &'static str {
        match self {
            Self::WindowsNativeZellij => "",
            Self::Electron | Self::UnixTmux => "n/a",
        }
    }
}

fn scaffold_manual_run(
    kind: ManualRunKind,
    date: &str,
    options: ManualRunScaffoldOptions,
) -> Result<PathBuf, String> {
    let ManualRunScaffoldOptions { output, os, rust, node, tmux, zellij, force } = options;
    let workspace_root = workspace_root();
    let manual_drafts_dir = workspace_root.join(MANUAL_DRAFTS_DIR);
    let template_path = workspace_root.join(MANUAL_RUNS_DIR).join("_template.md");
    let template = fs::read_to_string(&template_path)
        .map_err(|error| format!("failed to read {} - {error}", template_path.display()))?;
    let output_path = output
        .unwrap_or_else(|| manual_drafts_dir.join(format!("{}{date}.md", kind.file_prefix())));

    if output_path.exists() && !force {
        return Err(format!(
            "{} already exists - pass --force to overwrite",
            output_path.display()
        ));
    }

    let resolved_os = os.unwrap_or_else(detect_os_label);
    let resolved_rust = rust.unwrap_or_else(|| {
        probe_command(&["rustc", "--version"]).unwrap_or_else(|| "n/a".to_string())
    });
    let resolved_node = node.unwrap_or_else(|| {
        probe_command(&["node", "--version"]).unwrap_or_else(|| "n/a".to_string())
    });
    let resolved_tmux = match tmux {
        Some(value) => value,
        None => {
            if kind.default_tmux_value().is_empty() {
                probe_command(&["tmux", "-V"]).ok_or_else(|| {
                    "failed to detect tmux version - pass --tmux explicitly".to_string()
                })?
            } else {
                kind.default_tmux_value().to_string()
            }
        }
    };
    let resolved_zellij = match zellij {
        Some(value) => value,
        None => {
            if kind.default_zellij_value().is_empty() {
                probe_command(&["zellij", "--version"]).ok_or_else(|| {
                    "failed to detect Zellij version - pass --zellij explicitly".to_string()
                })?
            } else {
                kind.default_zellij_value().to_string()
            }
        }
    };

    let payload = template
        .replace(MANUAL_RUN_TEMPLATE_DATE_PLACEHOLDER, &format!("Date: {date}"))
        .replace(MANUAL_RUN_TEMPLATE_OS_PLACEHOLDER, &format!("OS: {resolved_os}"))
        .replace(
            MANUAL_RUN_TEMPLATE_CHECKLIST_PLACEHOLDER,
            &format!("Checklist: {}", kind.checklist_path()),
        )
        .replace(MANUAL_RUN_TEMPLATE_RUST_PLACEHOLDER, &format!("Rust: {resolved_rust}"))
        .replace(MANUAL_RUN_TEMPLATE_NODE_PLACEHOLDER, &format!("Node: {resolved_node}"))
        .replace("tmux: 3.x or n/a", &format!("tmux: {resolved_tmux}"))
        .replace("Zellij: 0.44.x or n/a", &format!("Zellij: {resolved_zellij}"))
        .replace("Result: pass", "Result: pending");

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {} - {error}", parent.display()))?;
    }
    fs::write(&output_path, payload)
        .map_err(|error| format!("failed to write {} - {error}", output_path.display()))?;
    Ok(output_path)
}

fn verify_recorded_pass(
    path: &Path,
    file_name: &str,
    contents: &str,
    expectation: ManualRunExpectation,
) -> Result<(), String> {
    assert_value(
        path.extension().and_then(|value| value.to_str()) == Some("md"),
        &format!("manual run artifact {} must be a markdown file", path.display()),
    )?;

    let date_value = require_line_value(contents, "Date: ", path)?;
    let checklist_value = require_line_value(contents, "Checklist: ", path)?;
    let _ = require_line_value(contents, "OS: ", path)?;
    let _ = require_line_value(contents, "Rust: ", path)?;
    let _ = require_line_value(contents, "Node: ", path)?;

    assert_value(
        contents.contains("Result: pass"),
        &format!("manual run artifact {} must say Result: pass", path.display()),
    )?;
    assert_value(
        contents.contains("## Scope"),
        &format!("manual run artifact {} is missing ## Scope", path.display()),
    )?;
    assert_value(
        contents.contains("## Findings"),
        &format!("manual run artifact {} is missing ## Findings", path.display()),
    )?;

    for template_placeholder in [
        MANUAL_RUN_TEMPLATE_DATE_PLACEHOLDER,
        MANUAL_RUN_TEMPLATE_OS_PLACEHOLDER,
        MANUAL_RUN_TEMPLATE_CHECKLIST_PLACEHOLDER,
        MANUAL_RUN_TEMPLATE_RUST_PLACEHOLDER,
        MANUAL_RUN_TEMPLATE_NODE_PLACEHOLDER,
    ] {
        assert_value(
            !contents.contains(template_placeholder),
            &format!(
                "manual run artifact {} still contains template placeholder: {template_placeholder}",
                path.display()
            ),
        )?;
    }

    assert_value(
        checklist_value == expectation.checklist_path,
        &format!(
            "manual run artifact {} has unexpected checklist: {}",
            path.display(),
            checklist_value
        ),
    )?;

    let expected_file_name = format!("{}{date_value}.md", expectation.file_prefix);
    assert_value(
        file_name == expected_file_name,
        &format!(
            "manual run artifact {} must match Date field with filename {}",
            path.display(),
            expected_file_name
        ),
    )?;

    if let Some((runtime_marker, runtime_placeholder)) = expectation.required_runtime_marker {
        let runtime_value = require_line_value(contents, runtime_marker, path)?;
        assert_value(
            !contents.contains(runtime_placeholder),
            &format!(
                "manual run artifact {} still contains template placeholder: {runtime_placeholder}",
                path.display()
            ),
        )?;
        assert_value(
            runtime_value != "n/a",
            &format!(
                "manual run artifact {} must record a real value for {runtime_marker}",
                path.display()
            ),
        )?;
    }

    Ok(())
}

fn require_line_value<'a>(contents: &'a str, prefix: &str, path: &Path) -> Result<&'a str, String> {
    let Some(line) = contents.lines().find(|line| line.starts_with(prefix)) else {
        return Err(format!(
            "manual run artifact {} is missing required marker: {prefix}",
            path.display()
        ));
    };
    let value = line[prefix.len()..].trim();
    assert_value(
        !value.is_empty(),
        &format!("manual run artifact {} has empty value for {prefix}", path.display()),
    )?;
    Ok(value)
}

fn stage_capi_package(out_dir: &Path) -> Result<PathBuf, String> {
    let capi_dir = workspace_root().join("crates/terminal-capi");
    let include_dir = out_dir.join("include");
    let lib_dir = out_dir.join("lib");
    let pkgconfig_dir = lib_dir.join("pkgconfig");
    let header_path = include_dir.join(CAPI_HEADER_NAME);
    let pkgconfig_path = pkgconfig_dir.join(format!("{CAPI_PKGCONFIG_NAME}.pc"));
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
    fs::create_dir_all(&pkgconfig_dir)
        .map_err(|error| format!("failed to create {} - {error}", pkgconfig_dir.display()))?;

    generate_header(&capi_dir, &header_path)?;
    copy_file(&capi_dir.join("README.md"), &out_dir.join("README.md"))?;
    copy_file(&cdylib_path, &lib_dir.join(cdylib_name))?;
    copy_file(&staticlib_path, &lib_dir.join(staticlib_name))?;
    write_pkgconfig(&pkgconfig_path, &package_version)?;

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
            "pkgConfig": format!("lib/pkgconfig/{CAPI_PKGCONFIG_NAME}.pc"),
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
    let pkgconfig_relative = exports
        .get("pkgConfig")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest exports.pkgConfig must be a string".to_string())?;
    let header_path = package_dir.join(header_relative);
    let cdylib_path = package_dir.join(cdylib_relative);
    let staticlib_path = package_dir.join(staticlib_relative);
    let pkgconfig_path = package_dir.join(pkgconfig_relative);
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
    assert_value(pkgconfig_path.is_file(), "staged pkg-config file is missing")?;
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
    let pkgconfig = fs::read_to_string(&pkgconfig_path)
        .map_err(|error| format!("failed to read {} - {error}", pkgconfig_path.display()))?;
    assert_value(
        pkgconfig.contains(&format!("Name: {CAPI_PKGCONFIG_NAME}")),
        "staged pkg-config file is missing expected package name",
    )?;
    assert_value(
        pkgconfig.contains(&format!("Version: {package_version}")),
        "staged pkg-config file is missing expected package version",
    )?;
    assert_value(
        pkgconfig.contains("Libs: -L${libdir} -lterminal_capi"),
        "staged pkg-config file is missing expected linker flags",
    )?;
    assert_value(
        pkgconfig.contains("Cflags: -I${includedir}"),
        "staged pkg-config file is missing expected include flags",
    )?;
    Ok(())
}

fn install_capi_package(package_dir: &Path, prefix: &Path) -> Result<PathBuf, String> {
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

fn verify_capi_install(prefix: &Path) -> Result<(), String> {
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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask workspace root should resolve")
        .to_path_buf()
}

fn export_sdk_runtime_types(out_dir: &Path) -> Result<PathBuf, String> {
    if out_dir.exists() {
        fs::remove_dir_all(out_dir)
            .map_err(|error| format!("failed to clean {} - {error}", out_dir.display()))?;
    }

    fs::create_dir_all(out_dir)
        .map_err(|error| format!("failed to create {} - {error}", out_dir.display()))?;

    terminal_node::export_typescript_bindings_to(out_dir).map_err(|error| {
        format!("failed to export runtime types to {} - {error}", out_dir.display())
    })?;

    Ok(out_dir.to_path_buf())
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

fn write_pkgconfig(pkgconfig_path: &Path, package_version: &str) -> Result<(), String> {
    let payload = format!(
        "prefix=${{pcfiledir}}/../..\nexec_prefix=${{prefix}}\nlibdir=${{exec_prefix}}/lib\nincludedir=${{prefix}}/include\n\nName: {CAPI_PKGCONFIG_NAME}\nDescription: Terminal platform C ABI package\nVersion: {package_version}\nLibs: -L${{libdir}} -l{CAPI_LIBRARY_BASENAME}\nCflags: -I${{includedir}}\n"
    );
    fs::write(pkgconfig_path, payload)
        .map_err(|error| format!("failed to write {} - {error}", pkgconfig_path.display()))
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

fn detect_os_label() -> String {
    match env::consts::OS {
        "macos" => "macOS".to_string(),
        "linux" => "Linux".to_string(),
        "windows" => "Windows".to_string(),
        other => other.to_string(),
    }
}

fn probe_command(command: &[&str]) -> Option<String> {
    let (program, args) = command.split_first()?;
    let output = ProcessCommand::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
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

fn copy_file_ensuring_parent(source: &Path, target: &Path) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| format!("target {} does not have a parent directory", target.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {} - {error}", parent.display()))?;
    copy_file(source, target)
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

#[cfg(test)]
mod tests {
    use super::verify_recorded_passes;
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_nanos(),
                Err(error) => panic!("failed to get test timestamp - {error}"),
            };
            let path = std::env::temp_dir()
                .join(format!("terminal-platform-xtask-test-{}-{timestamp}", std::process::id()));
            if let Err(error) = fs::create_dir_all(&path) {
                panic!("failed to create {} - {error}", path.display());
            }
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write_file(&self, relative_path: &str, contents: &str) {
            let path = self.path.join(relative_path);
            if let Err(error) = fs::write(&path, contents) {
                panic!("failed to write {} - {error}", path.display());
            }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn verify_recorded_passes_accepts_expected_artifacts() {
        let dir = TestDir::new();
        dir.write_file("README.md", "# Recorded Manual Passes\n");
        dir.write_file("_template.md", "# Run Title\n");
        dir.write_file(
            "electron-2026-04-20.md",
            "\
Date: 2026-04-20
OS: macOS 15.4
Checklist: crates/terminal-testing/manual/electron.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: n/a
Zellij: n/a

## Scope

Electron embed lifecycle and resize churn.

## Findings

no issues found

## Notes

none
",
        );
        dir.write_file(
            "unix-tmux-2026-04-20.md",
            "\
Date: 2026-04-20
OS: Ubuntu 24.04
Checklist: crates/terminal-testing/manual/tmux.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: 3.5a
Zellij: n/a

## Scope

tmux import and detach or reattach.

## Findings

no issues found

## Notes

none
",
        );
        dir.write_file(
            "windows-native-zellij-2026-04-20.md",
            "\
Date: 2026-04-20
OS: Windows 11 24H2
Checklist: crates/terminal-testing/manual/windows-native-zellij.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: n/a
Zellij: 0.44.1

## Scope

Native create or attach plus imported zellij mutation lane.

## Findings

no issues found

## Notes

none
",
        );

        if let Err(error) = verify_recorded_passes(dir.path()) {
            panic!("expected recorded passes to validate - {error}");
        }
    }

    #[test]
    fn verify_recorded_passes_rejects_template_placeholders() {
        let dir = TestDir::new();
        dir.write_file("README.md", "# Recorded Manual Passes\n");
        dir.write_file("_template.md", "# Run Title\n");
        dir.write_file(
            "electron-2026-04-20.md",
            "\
Date: YYYY-MM-DD
OS: macOS 15.4 / Ubuntu 24.04 / Windows 11 24H2
Checklist: crates/terminal-testing/manual/<checklist>.md
Result: pass

Rust: rustc 1.xx.x
Node: vxx.x.x
tmux: n/a
Zellij: n/a

## Scope

placeholder

## Findings

no issues found

## Notes

none
",
        );

        let error = match verify_recorded_passes(dir.path()) {
            Ok(()) => panic!("expected placeholder artifact to fail"),
            Err(error) => error,
        };
        assert!(error.contains("template placeholder"), "expected placeholder error, got: {error}");
    }

    #[test]
    fn verify_recorded_passes_rejects_missing_findings_section() {
        let dir = TestDir::new();
        dir.write_file("README.md", "# Recorded Manual Passes\n");
        dir.write_file("_template.md", "# Run Title\n");
        dir.write_file(
            "electron-2026-04-20.md",
            "\
Date: 2026-04-20
OS: macOS 15.4
Checklist: crates/terminal-testing/manual/electron.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: n/a
Zellij: n/a

## Scope

Electron embed lifecycle.

Findings:

no issues found

## Notes

none
",
        );

        let error = match verify_recorded_passes(dir.path()) {
            Ok(()) => panic!("expected findings section mismatch to fail"),
            Err(error) => error,
        };
        assert!(
            error.contains("missing ## Findings"),
            "expected findings section error, got: {error}"
        );
    }

    #[test]
    fn verify_recorded_passes_rejects_missing_runtime_value_for_required_pass() {
        let dir = TestDir::new();
        dir.write_file("README.md", "# Recorded Manual Passes\n");
        dir.write_file("_template.md", "# Run Title\n");
        dir.write_file(
            "electron-2026-04-20.md",
            "\
Date: 2026-04-20
OS: macOS 15.4
Checklist: crates/terminal-testing/manual/electron.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: n/a
Zellij: n/a

## Scope

Electron embed lifecycle and resize churn.

## Findings

no issues found

## Notes

none
",
        );
        dir.write_file(
            "unix-tmux-2026-04-20.md",
            "\
Date: 2026-04-20
OS: Ubuntu 24.04
Checklist: crates/terminal-testing/manual/tmux.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: n/a
Zellij: n/a

## Scope

tmux import and detach or reattach.

## Findings

no issues found

## Notes

none
",
        );
        dir.write_file(
            "windows-native-zellij-2026-04-20.md",
            "\
Date: 2026-04-20
OS: Windows 11 24H2
Checklist: crates/terminal-testing/manual/windows-native-zellij.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: n/a
Zellij: 0.44.1

## Scope

Native create or attach plus imported zellij mutation lane.

## Findings

no issues found

## Notes

none
",
        );

        let error = match verify_recorded_passes(dir.path()) {
            Ok(()) => panic!("expected runtime-specific n/a marker to fail"),
            Err(error) => error,
        };
        assert!(
            error.contains("must record a real value"),
            "expected runtime value error, got: {error}"
        );
    }
}
