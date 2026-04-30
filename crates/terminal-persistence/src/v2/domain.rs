use super::*;

pub(super) fn event_scope(session_id: &str, pane_id: Option<&str>) -> EventScope {
    match pane_id {
        Some(pane_id) => EventScope { kind: "pane".to_string(), id: pane_id.to_string() },
        None => EventScope { kind: "session".to_string(), id: session_id.to_string() },
    }
}

pub(super) struct EventScope {
    pub(super) kind: String,
    pub(super) id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BufferModeTransition {
    pub(super) action: &'static str,
    pub(super) target_buffer_kind: &'static str,
    pub(super) mode: i32,
    pub(super) byte_offset: i64,
    pub(super) byte_len: i64,
}

pub(super) fn detect_buffer_mode_transitions(payload: &[u8]) -> Vec<BufferModeTransition> {
    let mut transitions = Vec::new();
    let mut index = 0;
    while index + 3 < payload.len() {
        if payload[index] != 0x1b || payload[index + 1] != b'[' || payload[index + 2] != b'?' {
            index += 1;
            continue;
        }

        let params_start = index + 3;
        let mut cursor = params_start;
        while cursor < payload.len() && !is_csi_final_byte(payload[cursor]) {
            cursor += 1;
        }
        if cursor >= payload.len() {
            break;
        }

        let final_byte = payload[cursor];
        if matches!(final_byte, b'h' | b'l') {
            let action = if final_byte == b'h' { "enter" } else { "leave" };
            let target_buffer_kind = if final_byte == b'h' { "alternate" } else { "normal" };
            for mode in parse_private_mode_params(&payload[params_start..cursor]) {
                if matches!(mode, 47 | 1047 | 1049) {
                    transitions.push(BufferModeTransition {
                        action,
                        target_buffer_kind,
                        mode,
                        byte_offset: i64::try_from(index).unwrap_or(i64::MAX),
                        byte_len: i64::try_from(cursor + 1 - index).unwrap_or(i64::MAX),
                    });
                }
            }
        }
        index = cursor + 1;
    }
    transitions
}

pub(super) fn is_csi_final_byte(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

pub(super) fn parse_private_mode_params(params: &[u8]) -> Vec<i32> {
    params
        .split(|byte| matches!(*byte, b';' | b':'))
        .filter_map(|part| std::str::from_utf8(part).ok()?.parse::<i32>().ok())
        .collect()
}

pub(super) fn validate_positive_dimensions(
    rows: i32,
    cols: i32,
) -> Result<(), TerminalPersistenceV2Error> {
    if rows <= 0 || cols <= 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "terminal dimensions must be positive, got rows={rows}, cols={cols}"
        )));
    }
    Ok(())
}

pub(super) fn validate_optional_range(
    low: Option<i64>,
    high: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    match (low, high) {
        (Some(low), Some(high)) if low <= high => Ok(()),
        (None, None) => Ok(()),
        _ => Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} range must be either empty or fully populated"
        ))),
    }
}

pub(super) fn validate_optional_half_open_range(
    low: Option<i64>,
    high: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    match (low, high) {
        (Some(low), Some(high)) if low < high => Ok(()),
        (None, None) => Ok(()),
        _ => Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} range must be empty or half-open with low < high"
        ))),
    }
}

pub(super) fn validate_non_negative_seq(
    value: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    if let Some(value) = value
        && value < 0
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} must not be negative"
        )));
    }
    Ok(())
}

pub(super) fn checked_len(len: usize, label: &str) -> Result<i64, TerminalPersistenceV2Error> {
    i64::try_from(len).map_err(|_| {
        TerminalPersistenceV2Error::InvalidData(format!("{label} does not fit in i64"))
    })
}

pub(super) fn u64_to_i64(value: u64, label: &str) -> Result<i64, TerminalPersistenceV2Error> {
    i64::try_from(value).map_err(|_| {
        TerminalPersistenceV2Error::InvalidData(format!("{label} does not fit in i64"))
    })
}

pub(super) fn legacy_pane_high_water(saved: &SavedNativeSession) -> Value {
    let mut map = serde_json::Map::new();
    for screen in &saved.screens {
        map.insert(screen.pane_id.0.to_string(), Value::from(screen.sequence));
    }
    Value::Object(map)
}

pub(super) fn topology_pane_high_water_from_store(
    connection: &mut SqliteConnection,
    session_id: &str,
    topology: &TopologySnapshot,
) -> Result<Value, TerminalPersistenceV2Error> {
    let mut map = topology_pane_high_water_map(topology);
    if !map.is_empty() {
        let pane_ids = map.keys().cloned().collect::<Vec<_>>();
        let persisted_high_water = terminal_panes::table
            .filter(terminal_panes::session_id.eq(session_id))
            .filter(terminal_panes::id.eq_any(&pane_ids))
            .select((terminal_panes::id, terminal_panes::last_event_seq))
            .load::<(String, i64)>(connection)?;
        for (pane_id, last_event_seq) in persisted_high_water {
            if let Some(value) = map.get_mut(&pane_id) {
                *value = last_event_seq.max(0);
            }
        }
    }

    let mut output = serde_json::Map::new();
    for (pane_id, high_water_event_seq) in map {
        output.insert(pane_id, Value::from(high_water_event_seq));
    }
    Ok(Value::Object(output))
}

pub(super) fn topology_pane_high_water_map(topology: &TopologySnapshot) -> BTreeMap<String, i64> {
    let mut map = BTreeMap::new();
    for tab in &topology.tabs {
        collect_topology_pane_high_water(&tab.root, &mut map);
    }
    map
}

pub(super) fn collect_topology_pane_high_water(
    node: &terminal_mux_domain::PaneTreeNode,
    map: &mut BTreeMap<String, i64>,
) {
    match node {
        terminal_mux_domain::PaneTreeNode::Leaf { pane_id } => {
            map.entry(pane_id.0.to_string()).or_insert(0);
        }
        terminal_mux_domain::PaneTreeNode::Split(split) => {
            collect_topology_pane_high_water(&split.first, map);
            collect_topology_pane_high_water(&split.second, map);
        }
    }
}

pub(super) fn stream_cursor_id(pane_id: &str, stream_id: &str) -> String {
    format!("stream-cursor-{}", blake3_hash_text(&format!("{pane_id}\0{stream_id}")))
}

pub(super) fn stream_capture_source_kind(pane_id: &str, stream_id: &str) -> String {
    format!("stream-segment-{}", blake3_hash_text(&format!("{pane_id}\0{stream_id}")))
}

pub(super) fn payload_schema_id_for_journal_event(event_type: &str) -> &'static str {
    match event_type {
        "terminal_input" | "terminal_paste_input" => PAYLOAD_SCHEMA_UI_INPUT_V1,
        "history_gap" => PAYLOAD_SCHEMA_HISTORY_GAP_V1,
        _ => PAYLOAD_SCHEMA_JOURNAL_EVENT_V1,
    }
}

pub(super) fn delivery_offset_id(
    client_id: &str,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> String {
    format!(
        "delivery-offset-{}",
        blake3_hash_text(&format!("{client_id}\0{session_id}\0{pane_id}\0{stream_id}"))
    )
}

pub(super) fn normalize_outbox_dedupe_key(value: &str) -> String {
    format!("blake3:{}", blake3_hash_text(value))
}

pub(super) fn ui_input_capture_source_kind(pane_id: &str) -> String {
    format!("ui-input-{}", blake3_hash_text(pane_id))
}

pub(super) fn stable_ui_command_block_id(
    session_id: &str,
    pane_id: &str,
    source_event_id_hash: &str,
) -> String {
    format!(
        "command-block-{}",
        blake3_hash_text(&format!("{session_id}\0{pane_id}\0{source_event_id_hash}"))
    )
}

pub(super) fn stable_history_id(
    scope_kind: &str,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    command_hash: &str,
) -> String {
    let material = format!(
        "{}\0{}\0{}\0{}",
        scope_kind,
        session_id.unwrap_or_default(),
        pane_id.unwrap_or_default(),
        command_hash
    );
    format!("command-history-{}", blake3_hash_text(&material))
}

pub(super) fn stable_search_document_id(
    session_id: &str,
    pane_id: Option<&str>,
    command_block_id: Option<&str>,
    source_hash: &str,
) -> String {
    let material = format!(
        "{}\0{}\0{}\0{}",
        session_id,
        pane_id.unwrap_or_default(),
        command_block_id.unwrap_or_default(),
        source_hash
    );
    format!("search-document-{}", blake3_hash_text(&material))
}

pub(super) fn command_text_from_ui_input(data: &str) -> Option<String> {
    let trimmed_end = data.trim_end_matches(['\r', '\n']);
    if trimmed_end.len() == data.len() {
        return None;
    }
    let command = trimmed_end.trim();
    if command.is_empty() { None } else { Some(command.to_string()) }
}

pub fn shell_metadata_profile(
    launch: Option<&ShellLaunchSpec>,
    explicit_shell_kind: Option<&str>,
) -> ShellMetadataProfile {
    let shell_kind = explicit_shell_kind
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_ascii_lowercase())
        .or_else(|| launch.and_then(|launch| infer_shell_kind_from_program(&launch.program)));
    let windows_profile = matches!(shell_kind.as_deref(), Some("cmd" | "powershell" | "pwsh"));
    let command_boundary_confidence = match shell_kind.as_deref() {
        Some("cmd" | "powershell" | "pwsh") => "high",
        Some("bash" | "sh" | "zsh" | "fish") => "high",
        Some(_) => "medium",
        None => "unknown",
    }
    .to_string();
    let cwd_source = if launch.and_then(|launch| launch.cwd.as_ref()).is_some() {
        "launch_cwd"
    } else {
        "unknown"
    }
    .to_string();
    let input_terminator = if windows_profile { "cr" } else { "lf_or_cr" }.to_string();

    ShellMetadataProfile {
        shell_kind,
        command_boundary_confidence,
        cwd_source,
        input_terminator,
        windows_profile,
    }
}

pub(super) fn infer_shell_kind_from_program(program: &str) -> Option<String> {
    let normalized = program.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or(program).to_ascii_lowercase();
    let stem = file_name.strip_suffix(".exe").unwrap_or(&file_name);
    match stem {
        "cmd" | "cmd32" | "cmd64" => Some("cmd".to_string()),
        "powershell" => Some("powershell".to_string()),
        "pwsh" => Some("pwsh".to_string()),
        "bash" => Some("bash".to_string()),
        "sh" => Some("sh".to_string()),
        "zsh" => Some("zsh".to_string()),
        "fish" => Some("fish".to_string()),
        _ => None,
    }
}

pub(super) fn new_id() -> String {
    Uuid::now_v7().to_string()
}

pub(super) fn bool_to_int(value: bool) -> i32 {
    i32::from(value)
}

pub(super) fn json_metadata(
    value: &Option<Value>,
) -> Result<Option<String>, TerminalPersistenceV2Error> {
    value.as_ref().map(serde_json::to_string).transpose().map_err(Into::into)
}

pub(super) fn merge_json_field(
    existing: Option<&str>,
    field: &str,
    value: Value,
) -> Result<Option<String>, TerminalPersistenceV2Error> {
    let mut root =
        existing.map(serde_json::from_str).transpose()?.unwrap_or_else(|| serde_json::json!({}));
    if !root.is_object() {
        root = serde_json::json!({ "legacy_json_value": root });
    }
    root.as_object_mut().expect("root is normalized to an object").insert(field.to_string(), value);
    Ok(Some(serde_json::to_string(&root)?))
}

pub(super) fn blake3_hash_bytes(value: &[u8]) -> String {
    blake3::hash(value).to_hex().to_string()
}

pub(super) fn blake3_hash_text(value: &str) -> String {
    blake3_hash_bytes(value.as_bytes())
}

pub(super) fn local_keyed_command_hash(
    connection: &mut SqliteConnection,
    command_text: &str,
) -> Result<String, TerminalPersistenceV2Error> {
    let key_seed = load_or_create_command_hash_key_seed(connection)?;
    let key_hash = blake3::hash(key_seed.as_bytes());
    let key = *key_hash.as_bytes();
    Ok(blake3::keyed_hash(&key, command_text.as_bytes()).to_hex().to_string())
}

pub(super) fn load_or_create_command_hash_key_seed(
    connection: &mut SqliteConnection,
) -> Result<String, TerminalPersistenceV2Error> {
    let notes = terminal_db_identity::table
        .filter(terminal_db_identity::id.eq(1))
        .select(terminal_db_identity::notes)
        .first::<Option<String>>(connection)?;

    let mut notes_value = parse_identity_notes(notes.as_deref());
    if let Some(key_seed) = command_hash_key_seed_from_notes(&notes_value) {
        return Ok(key_seed.to_string());
    }

    let key_seed = format!("command-history-hash-key-v1:{}:{}", Uuid::new_v4(), Uuid::new_v4());
    if !notes_value.is_object() {
        let previous = notes_value;
        notes_value = serde_json::json!({ "legacy_notes_value": previous });
    }
    let object = notes_value.as_object_mut().ok_or_else(|| {
        TerminalPersistenceV2Error::InvalidData("db identity notes are not an object".to_string())
    })?;
    let privacy_value =
        object.entry("privacy".to_string()).or_insert_with(|| serde_json::json!({}));
    if !privacy_value.is_object() {
        let previous = privacy_value.take();
        *privacy_value = serde_json::json!({ "legacy_privacy_value": previous });
    }
    let privacy = privacy_value.as_object_mut().ok_or_else(|| {
        TerminalPersistenceV2Error::InvalidData(
            "db identity privacy notes are not an object".to_string(),
        )
    })?;
    privacy.insert("command_history_hash_key_seed_v1".to_string(), Value::String(key_seed.clone()));
    privacy.insert(
        "command_history_hash_algorithm".to_string(),
        Value::String(COMMAND_HASH_ALGORITHM.to_string()),
    );
    privacy.insert(
        "command_history_hash_scope".to_string(),
        Value::String(COMMAND_HASH_SCOPE.to_string()),
    );

    diesel::update(terminal_db_identity::table.filter(terminal_db_identity::id.eq(1)))
        .set((
            terminal_db_identity::updated_at_ms.eq(current_time_ms()),
            terminal_db_identity::notes.eq(Some(serde_json::to_string(&notes_value)?)),
        ))
        .execute(connection)?;

    Ok(key_seed)
}

pub(super) fn parse_identity_notes(notes: Option<&str>) -> Value {
    match notes {
        Some(raw) => serde_json::from_str(raw)
            .unwrap_or_else(|_| serde_json::json!({ "legacy_notes_raw": raw })),
        None => serde_json::json!({}),
    }
}

pub(super) fn command_hash_key_seed_from_notes(notes: &Value) -> Option<&str> {
    notes
        .get("privacy")?
        .get("command_history_hash_key_seed_v1")?
        .as_str()
        .filter(|value| !value.is_empty())
}

pub(super) fn blake3_hash_file(path: &Path) -> Result<String, TerminalPersistenceV2Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

pub(super) fn file_len_i64(path: &Path) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    fs::metadata(path).ok().map(|metadata| u64_to_i64(metadata.len(), "file size")).transpose()
}

pub(super) fn prepare_vacuum_backup_target(
    source_path: &Path,
    target_path: &Path,
) -> Result<PathBuf, TerminalPersistenceV2Error> {
    let file_name = target_path.file_name().ok_or_else(|| {
        TerminalPersistenceV2Error::InvalidData(format!(
            "backup target must include a file name: {}",
            target_path.display()
        ))
    })?;
    let parent = target_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let source_canonical = source_path.canonicalize()?;
    let target_absolute = parent.canonicalize()?.join(file_name);
    let forbidden_targets = [
        source_canonical.clone(),
        sqlite_sidecar_path(&source_canonical, "-wal"),
        sqlite_sidecar_path(&source_canonical, "-shm"),
    ];
    if forbidden_targets
        .iter()
        .any(|forbidden| paths_equal_for_platform(forbidden, &target_absolute))
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "backup target cannot point at the live database or SQLite sidecar: {}",
            target_absolute.display()
        )));
    }
    if target_absolute.exists() {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "backup target already exists: {}",
            target_absolute.display()
        )));
    }

    Ok(target_absolute)
}

pub(super) fn paths_equal_for_platform(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

pub(super) fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StoragePressureClassification {
    pub(super) state: &'static str,
    pub(super) action_taken: &'static str,
    pub(super) reason: &'static str,
    pub(super) db_over_budget: bool,
    pub(super) wal_over_budget: bool,
}

pub(super) fn classify_storage_pressure(
    db_file_bytes: Option<i64>,
    wal_file_bytes: Option<i64>,
    config: StoragePressureConfig,
) -> StoragePressureClassification {
    let db_over_budget = config.db_warning_bytes > 0
        && db_file_bytes.is_some_and(|value| value >= config.db_warning_bytes);
    let wal_over_budget = config.wal_warning_bytes > 0
        && wal_file_bytes.is_some_and(|value| value >= config.wal_warning_bytes);
    let (state, action_taken, reason) = match (db_over_budget, wal_over_budget) {
        (true, true) => ("warning", "checkpoint_and_warn", "db_and_wal_file_size_over_budget"),
        (false, true) => ("warning", "checkpoint_recommended", "wal_file_size_over_budget"),
        (true, false) => ("warning", "warn_only", "db_file_size_over_budget"),
        (false, false) => ("ok", "none", "manual_probe"),
    };

    StoragePressureClassification { state, action_taken, reason, db_over_budget, wal_over_budget }
}

pub(super) fn validate_storage_pressure_domain(
    state: &str,
    action_taken: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const STATES: &[&str] = &["ok", "warning", "degraded", "full", "unknown"];
    const ACTIONS: &[&str] = &[
        "none",
        "warn_only",
        "checkpoint_recommended",
        "checkpoint_and_warn",
        "degrade_with_gap",
        "fail_closed",
    ];
    if !STATES.contains(&state) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown storage pressure state: {state}"
        )));
    }
    if !ACTIONS.contains(&action_taken) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown storage pressure action: {action_taken}"
        )));
    }
    Ok(())
}

pub(super) fn validate_crypto_key_domain(
    key_kind: &str,
    protection_kind: &str,
    state: Option<&str>,
) -> Result<(), TerminalPersistenceV2Error> {
    const KEY_KINDS: &[&str] = &["database_key", "export_key", "artifact_key"];
    const PROTECTION_KINDS: &[&str] = &[
        "windows_credential_manager",
        "dpapi_user",
        "dpapi_machine",
        "macos_keychain",
        "linux_secret_service",
        "test_plaintext",
    ];
    const STATES: &[&str] = &["active", "rotating", "disabled", "destroyed", "unavailable"];
    if !KEY_KINDS.contains(&key_kind) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key kind: {key_kind}"
        )));
    }
    if !PROTECTION_KINDS.contains(&protection_kind) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key protection kind: {protection_kind}"
        )));
    }
    if let Some(state) = state
        && !STATES.contains(&state)
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key state: {state}"
        )));
    }
    Ok(())
}

pub(super) fn validate_crypto_key_event_domain(
    event_kind: &str,
    status: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const EVENT_KINDS: &[&str] = &[
        "created",
        "unlocked",
        "lock_failed",
        "rotated",
        "destroy_requested",
        "destroyed",
        "recovery_failed",
    ];
    const STATUSES: &[&str] = &["succeeded", "failed", "skipped"];
    if !EVENT_KINDS.contains(&event_kind) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key event kind: {event_kind}"
        )));
    }
    if !STATUSES.contains(&status) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key event status: {status}"
        )));
    }
    Ok(())
}

pub(super) fn validate_crypto_key_ref(key_ref: &str) -> Result<(), TerminalPersistenceV2Error> {
    if key_ref.trim().is_empty() {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "crypto key_ref must not be empty".to_string(),
        ));
    }
    if key_ref.len() > 512 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "crypto key_ref must stay an opaque short reference, not key material".to_string(),
        ));
    }
    if key_ref.contains('\n') || key_ref.contains('\r') {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "crypto key_ref must be a single-line opaque reference".to_string(),
        ));
    }
    let lower = key_ref.to_ascii_lowercase();
    if lower.contains("begin ") || lower.contains("private key") || lower.contains("secret key") {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "crypto key_ref must not contain key material".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_external_artifact_domain(
    artifact_kind: &str,
    state: Option<&str>,
    encryption_state: Option<&str>,
) -> Result<(), TerminalPersistenceV2Error> {
    const ARTIFACT_KINDS: &[&str] =
        &["backup_file", "large_segment", "export_file", "support_bundle", "future_external_store"];
    const STATES: &[&str] =
        &["planned", "available", "verified", "missing", "deleted", "quarantined"];
    const ENCRYPTION_STATES: &[&str] = &["plaintext", "encrypted", "redacted", "crypto_erased"];
    if !ARTIFACT_KINDS.contains(&artifact_kind) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown external artifact kind: {artifact_kind}"
        )));
    }
    if let Some(state) = state
        && !STATES.contains(&state)
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown external artifact state: {state}"
        )));
    }
    if let Some(encryption_state) = encryption_state
        && !ENCRYPTION_STATES.contains(&encryption_state)
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown external artifact encryption state: {encryption_state}"
        )));
    }
    Ok(())
}

pub(super) fn validate_external_artifact_ref(
    artifact_ref: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    if artifact_ref.trim().is_empty() {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "external artifact ref must not be empty".to_string(),
        ));
    }
    if artifact_ref.len() > 2_048 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "external artifact ref is too long".to_string(),
        ));
    }
    if artifact_ref.contains('\n') || artifact_ref.contains('\r') {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "external artifact ref must be single-line before hashing".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_external_artifact_target_ref(
    artifact_ref: &str,
    source_db_path: &Path,
) -> Result<(), TerminalPersistenceV2Error> {
    let Some(target_path) = path_like_artifact_ref(artifact_ref) else {
        return Ok(());
    };
    let source_canonical = source_db_path.canonicalize()?;
    let Some(target_normalized) = normalize_artifact_target_path(&target_path) else {
        return Ok(());
    };
    let forbidden_targets = [
        source_canonical.clone(),
        sqlite_sidecar_path(&source_canonical, "-wal"),
        sqlite_sidecar_path(&source_canonical, "-shm"),
    ];
    if forbidden_targets
        .iter()
        .any(|forbidden| paths_equal_for_platform(forbidden, &target_normalized))
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "external artifact ref cannot point at the live database or SQLite sidecar: {}",
            target_normalized.display()
        )));
    }
    Ok(())
}

pub(super) fn validate_ai_action_kind(value: &str) -> Result<(), TerminalPersistenceV2Error> {
    const ACTION_KINDS: &[&str] =
        &["send_input", "rerun_command", "export", "share", "delete", "open_link"];
    if !ACTION_KINDS.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown AI action kind: {value}"
        )));
    }
    Ok(())
}

pub(super) fn path_like_artifact_ref(artifact_ref: &str) -> Option<PathBuf> {
    let trimmed = artifact_ref.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") {
        return None;
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute()
        || trimmed.starts_with('.')
        || trimmed.contains('\\')
        || trimmed.contains('/')
        || looks_like_windows_drive_path(trimmed)
    {
        Some(path)
    } else {
        None
    }
}

pub(super) fn looks_like_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && bytes[0].is_ascii_alphabetic()
}

pub(super) fn normalize_artifact_target_path(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = path.canonicalize() {
        return Some(canonical);
    }
    let file_name = path.file_name()?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    parent.canonicalize().ok().map(|parent| parent.join(file_name))
}

pub(super) fn is_storage_full_like_error(error: &TerminalPersistenceV2Error) -> bool {
    match error {
        TerminalPersistenceV2Error::Query(DieselError::DatabaseError(_, info)) => {
            let message = info.message().to_ascii_lowercase();
            message.contains("sqlite_full")
                || message.contains("database or disk is full")
                || message.contains("disk is full")
                || message.contains("database is full")
        }
        TerminalPersistenceV2Error::Io(error) => {
            let message = error.to_string().to_ascii_lowercase();
            message.contains("disk full")
                || message.contains("disk is full")
                || message.contains("not enough space")
        }
        _ => false,
    }
}

pub(super) fn validate_capture_semantics_domain(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const CAPTURE_SEMANTICS: &[&str] = &[
        "raw_vt_stream",
        "rendered_ansi_stream",
        "rendered_plaintext_snapshot",
        "mux_structured_surface",
        "imported_text",
        "ui_input",
    ];
    if !CAPTURE_SEMANTICS.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown capture semantics: {value}"
        )));
    }
    Ok(())
}

pub(super) fn validate_capture_strategy_domain(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const CAPTURE_STRATEGIES: &[&str] = &[
        "raw_stream",
        "rendered_stream",
        "rendered_snapshot",
        "mux_structured",
        "imported_snapshot",
        "ui_input",
        "unknown",
    ];
    if !CAPTURE_STRATEGIES.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown capture strategy: {value}"
        )));
    }
    Ok(())
}

pub(super) fn validate_command_boundary_confidence_domain(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const CONFIDENCE_LEVELS: &[&str] = &["verified", "high", "medium", "low", "none", "unknown"];
    if !CONFIDENCE_LEVELS.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown command boundary confidence: {value}"
        )));
    }
    Ok(())
}

pub(super) fn validate_backend_probe_status_domain(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const PROBE_STATUSES: &[&str] = &["passed", "failed", "partial", "stale"];
    if !PROBE_STATUSES.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown backend probe status: {value}"
        )));
    }
    Ok(())
}

pub(super) fn validate_backend_capability_stale_reason(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const REASONS: &[&str] = &[
        "backend_version_changed",
        "backend_binary_path_changed",
        "backend_config_changed",
        "probe_failed",
        "manual_invalidation",
    ];
    if !REASONS.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown backend capability stale reason: {value}"
        )));
    }
    Ok(())
}

pub(super) fn path_hash(path: &Path) -> String {
    blake3_hash_text(&path.to_string_lossy())
}

pub(super) fn privacy_manifest(kind: &str, include_raw: bool, session_id: Option<&str>) -> Value {
    let included_classes = if include_raw {
        vec![
            "class_public_diagnostic",
            "class_local_metadata",
            "class_user_context",
            "class_sensitive_content",
        ]
    } else {
        vec!["class_public_diagnostic", "class_local_metadata", "class_user_context_redacted"]
    };
    let excluded_classes = if include_raw {
        vec!["class_secret_material"]
    } else {
        vec!["class_sensitive_content", "class_secret_material"]
    };
    serde_json::json!({
        "kind": kind,
        "include_raw": include_raw,
        "session_id": session_id,
        "included_classes": included_classes,
        "excluded_classes": excluded_classes,
        "raw_terminal_output": include_raw,
        "raw_command_text": include_raw,
        "prompt_injection_text_is_data": true,
    })
}

pub(super) fn limit_text_preview(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub(super) fn redact_terminal_text(value: &str) -> String {
    let mut redacted = Vec::new();
    for token in value.split_whitespace() {
        redacted.push(redact_token(token));
    }
    redacted.join(" ")
}

pub(super) fn detect_prompt_injection_pattern(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    const PATTERNS: &[(&str, &str)] = &[
        ("ignore previous instructions", "ignore_previous_instructions"),
        ("ignore all previous instructions", "ignore_previous_instructions"),
        ("system prompt", "system_prompt_request"),
        ("developer message", "developer_message_request"),
        ("you are chatgpt", "model_identity_override"),
        ("do not follow", "instruction_override"),
        ("forget your instructions", "instruction_override"),
    ];
    PATTERNS.iter().find_map(|(needle, pattern)| lower.contains(needle).then_some(*pattern))
}

pub(super) fn redact_token(token: &str) -> String {
    const KEY_PREFIXES: [&str; 8] = [
        "password=",
        "passwd=",
        "pwd=",
        "token=",
        "access_token=",
        "api_key=",
        "apikey=",
        "secret=",
    ];
    let lower = token.to_ascii_lowercase();
    if lower == "bearer" {
        return token.to_string();
    }
    if token.len() >= 24
        && token.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        && token.chars().any(|ch| ch.is_ascii_digit())
        && token.chars().any(|ch| ch.is_ascii_alphabetic())
    {
        return "[redacted-secret]".to_string();
    }
    for prefix in KEY_PREFIXES {
        if lower.starts_with(prefix) {
            return format!("{}[redacted]", &token[..prefix.len().min(token.len())]);
        }
    }
    token.to_string()
}

pub(super) fn current_time_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => match i64::try_from(duration.as_millis()) {
            Ok(value) => value,
            Err(_) => i64::MAX,
        },
        Err(_) => 0,
    }
}
