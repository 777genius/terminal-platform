use crate::support::assert_contains_all;

pub(crate) fn verify_node_package_scripts(
    stage_script: &str,
    build_script: &str,
    pack_script: &str,
    verify_script: &str,
) -> Result<(), String> {
    assert_contains_all(
        stage_script,
        "Node package stage script guardrails",
        &[
            "assertSafeOutputDir(outDir)",
            "Refusing to stage package into unsafe output directory",
            "options.out = readFlagValue(argv, index, arg)",
            "options.addon = readFlagValue(argv, index, arg)",
            "Missing value for ${flag}",
        ],
    )?;
    assert_contains_all(
        build_script,
        "Node package build script guardrails",
        &["options.out = readFlagValue(argv, index, arg)", "Missing value for ${flag}"],
    )?;
    assert_contains_all(
        pack_script,
        "Node package pack script guardrails",
        &[
            "options.out = path.resolve(readFlagValue(argv, index, arg))",
            "Missing value for ${flag}",
            "npm_config_cache",
            "nodePackageManager()",
            "process.platform === \"win32\" ? \"npm.cmd\" : \"npm\"",
            "shell: packageManagerShell()",
            "process.platform === \"win32\" ? process.env.ComSpec ?? true : false",
            "npm pack failed to launch -",
            "signal ${packResult.signal ?? \"<none>\"}",
        ],
    )?;
    assert_contains_all(
        verify_script,
        "Node package verify script guardrails",
        &["options.packageDir = readFlagValue(argv, index, arg)", "Missing value for ${flag}"],
    )?;

    Ok(())
}
