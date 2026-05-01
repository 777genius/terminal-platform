use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    constants::*,
    support::{
        assert_value, contains_unresolved_manual_placeholder, detect_os_label, probe_command,
        workspace_root,
    },
};

#[derive(Clone, Copy)]
pub(crate) struct ManualRunExpectation {
    pub(crate) file_prefix: &'static str,
    pub(crate) checklist_path: &'static str,
    pub(crate) required_runtime_marker: Option<(&'static str, &'static str)>,
}

pub(crate) const REQUIRED_MANUAL_RUNS: [ManualRunExpectation; 3] = [
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

#[derive(Clone, Copy)]
pub(crate) enum ManualRunKind {
    Electron,
    UnixTmux,
    WindowsNativeZellij,
}

pub(crate) struct ManualRunScaffoldOptions {
    pub(crate) output: Option<PathBuf>,
    pub(crate) os: Option<String>,
    pub(crate) rust: Option<String>,
    pub(crate) node: Option<String>,
    pub(crate) tmux: Option<String>,
    pub(crate) zellij: Option<String>,
    pub(crate) workflow: Option<String>,
    pub(crate) job: Option<String>,
    pub(crate) force: bool,
}

pub(crate) fn parse_manual_run_kind(value: &str) -> Result<ManualRunKind, String> {
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

pub(crate) fn scaffold_manual_run(
    kind: ManualRunKind,
    date: &str,
    options: ManualRunScaffoldOptions,
) -> Result<PathBuf, String> {
    let ManualRunScaffoldOptions { output, os, rust, node, tmux, zellij, workflow, job, force } =
        options;
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
    let payload = if matches!(kind, ManualRunKind::WindowsNativeZellij) {
        let payload = payload.replacen(
            "\n## Scope",
            "\nWorkflow: fill from workflow log\nJob: fill from workflow log\n\n## Scope",
            1,
        );
        let payload = if let Some(workflow) = workflow {
            payload.replace("Workflow: fill from workflow log", &format!("Workflow: {workflow}"))
        } else {
            payload
        };
        if let Some(job) = job {
            payload.replace("Job: fill from workflow log", &format!("Job: {job}"))
        } else {
            payload
        }
    } else {
        payload
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {} - {error}", parent.display()))?;
    }
    fs::write(&output_path, payload)
        .map_err(|error| format!("failed to write {} - {error}", output_path.display()))?;
    Ok(output_path)
}

pub(crate) fn verify_recorded_pass(
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
    assert_value(
        contents.contains("## Notes"),
        &format!("manual run artifact {} is missing ## Notes", path.display()),
    )?;
    assert_value(
        !contents.contains("Result: pending"),
        &format!("manual run artifact {} still says Result: pending", path.display()),
    )?;
    assert_value(
        !contents.contains("TODO") && !contents.contains("TBD"),
        &format!("manual run artifact {} still contains TODO/TBD placeholders", path.display()),
    )?;
    assert_value(
        !contains_unresolved_manual_placeholder(contents),
        &format!(
            "manual run artifact {} still contains unresolved placeholder text",
            path.display()
        ),
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

    for section in ["## Scope", "## Findings", "## Notes"] {
        assert_value(
            !section_body(contents, section).trim().is_empty(),
            &format!("manual run artifact {} has an empty {section} section", path.display()),
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

    if expectation.file_prefix == "windows-native-zellij-" {
        let workflow_value = require_line_value(contents, "Workflow: ", path)?;
        let job_value = require_line_value(contents, "Job: ", path)?;
        assert_value(
            workflow_value.contains("https://github.com/")
                && workflow_value.contains("/actions/runs/"),
            &format!(
                "manual run artifact {} must record the exact hosted workflow URL",
                path.display()
            ),
        )?;
        assert_value(
            job_value.to_ascii_lowercase().contains("windows-v1"),
            &format!(
                "manual run artifact {} must record the exact hosted windows-v1 job",
                path.display()
            ),
        )?;
    }

    Ok(())
}

fn section_body<'a>(contents: &'a str, heading: &str) -> &'a str {
    let Some((_, tail)) = contents.split_once(heading) else {
        return "";
    };
    tail.split_once("\n## ").map_or(tail, |(body, _)| body)
}

pub(crate) fn require_line_value<'a>(
    contents: &'a str,
    prefix: &str,
    path: &Path,
) -> Result<&'a str, String> {
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
