use super::super::*;
use super::*;

pub(in crate::v2) fn new_id() -> String {
    Uuid::now_v7().to_string()
}

pub(in crate::v2) fn bool_to_int(value: bool) -> i32 {
    i32::from(value)
}

pub(in crate::v2) fn json_metadata(
    value: &Option<Value>,
) -> Result<Option<String>, TerminalPersistenceV2Error> {
    value.as_ref().map(serde_json::to_string).transpose().map_err(Into::into)
}

pub(in crate::v2) fn merge_json_field(
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

pub(in crate::v2) fn blake3_hash_bytes(value: &[u8]) -> String {
    blake3::hash(value).to_hex().to_string()
}

pub(in crate::v2) fn blake3_hash_text(value: &str) -> String {
    blake3_hash_bytes(value.as_bytes())
}

pub(in crate::v2) fn local_keyed_command_hash(
    connection: &mut SqliteConnection,
    command_text: &str,
) -> Result<String, TerminalPersistenceV2Error> {
    let key_seed = load_or_create_command_hash_key_seed(connection)?;
    let key_hash = blake3::hash(key_seed.as_bytes());
    let key = *key_hash.as_bytes();
    Ok(blake3::keyed_hash(&key, command_text.as_bytes()).to_hex().to_string())
}

pub(in crate::v2) fn load_or_create_command_hash_key_seed(
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

pub(in crate::v2) fn parse_identity_notes(notes: Option<&str>) -> Value {
    match notes {
        Some(raw) => serde_json::from_str(raw)
            .unwrap_or_else(|_| serde_json::json!({ "legacy_notes_raw": raw })),
        None => serde_json::json!({}),
    }
}

pub(in crate::v2) fn command_hash_key_seed_from_notes(notes: &Value) -> Option<&str> {
    notes
        .get("privacy")?
        .get("command_history_hash_key_seed_v1")?
        .as_str()
        .filter(|value| !value.is_empty())
}
