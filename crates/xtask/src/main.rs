use std::env;

mod capi;
mod command;
mod constants;
mod manual;
mod readiness;
mod support;

use capi::{
    export_sdk_runtime_types, install_capi_package, stage_capi_package, verify_capi_install,
    verify_capi_package,
};
use command::{Command, parse_command};
pub(crate) use manual::{ManualRunScaffoldOptions, scaffold_manual_run};
use readiness::verify_v1_readiness;

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
        Command::ScaffoldManualRun {
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
        } => {
            let output_path = scaffold_manual_run(
                kind,
                &date,
                ManualRunScaffoldOptions {
                    output,
                    os,
                    rust,
                    node,
                    tmux,
                    zellij,
                    workflow,
                    job,
                    force,
                },
            )?;
            println!("{}", output_path.display());
            Ok(())
        }
    }
}

#[cfg(test)]
pub(crate) use manual::ManualRunKind;
#[cfg(test)]
pub(crate) use readiness::{
    verify_manual_qa_scope, verify_node_package_scripts, verify_recorded_passes,
    verify_v1_release_configs, verify_v1_workflows, verify_windows_conpty_vendor_patch,
    verify_windows_zellij_package_smoke,
};

#[cfg(test)]
mod tests;
