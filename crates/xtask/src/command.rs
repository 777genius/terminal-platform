use std::path::PathBuf;

use crate::{
    manual::{ManualRunKind, parse_manual_run_kind},
    support::workspace_root,
};

pub(crate) enum Command {
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
        workflow: Option<String>,
        job: Option<String>,
        force: bool,
    },
}

pub(crate) fn parse_command(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
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
            let mut workflow = None;
            let mut job = None;
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
                    "--workflow" => {
                        workflow = Some(
                            args.next()
                                .ok_or_else(|| "missing value for --workflow".to_string())?,
                        );
                    }
                    "--job" => {
                        job =
                            Some(args.next().ok_or_else(|| "missing value for --job".to_string())?);
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
                workflow,
                job,
                force,
            })
        }
        other => Err(format!("unsupported xtask command: {other}")),
    }
}
