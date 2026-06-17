use terminal_backend_api::BackendError;

#[must_use]
pub fn probe_surface_code(
    version_output: &str,
    root_help: Option<&str>,
    action_help: Option<&str>,
) -> u8 {
    match crate::probe::ZellijProbe::parse(version_output, root_help, action_help, None, None)
        .surface
    {
        crate::probe::ZellijSurface::LegacyCli043 => 1,
        crate::probe::ZellijSurface::RichCli044Plus => 2,
        crate::probe::ZellijSurface::Unknown => 0,
    }
}

pub fn parse_tabs_json_len(output: &str) -> Result<usize, BackendError> {
    Ok(crate::rows::parse_tabs_json(output)?.len())
}

pub fn parse_panes_json_len(output: &str) -> Result<usize, BackendError> {
    Ok(crate::rows::parse_panes_json(output)?.len())
}
