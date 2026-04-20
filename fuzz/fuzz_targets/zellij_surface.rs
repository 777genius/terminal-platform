#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct ZellijInput {
    version_output: String,
    root_help: Option<String>,
    action_help: Option<String>,
    tabs_json: String,
    panes_json: String,
}

fuzz_target!(|input: ZellijInput| {
    let _ = terminal_backend_zellij::__fuzz::probe_surface_code(
        &input.version_output,
        input.root_help.as_deref(),
        input.action_help.as_deref(),
    );
    let _ = terminal_backend_zellij::__fuzz::parse_tabs_json_len(&input.tabs_json);
    let _ = terminal_backend_zellij::__fuzz::parse_panes_json_len(&input.panes_json);
});
