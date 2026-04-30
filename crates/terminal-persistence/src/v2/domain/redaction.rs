use super::super::*;
use super::*;

pub(in crate::v2) fn path_hash(path: &Path) -> String {
    blake3_hash_text(&path.to_string_lossy())
}

pub(in crate::v2) fn privacy_manifest(
    kind: &str,
    include_raw: bool,
    session_id: Option<&str>,
) -> Value {
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

pub(in crate::v2) fn limit_text_preview(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub(in crate::v2) fn redact_terminal_text(value: &str) -> String {
    let mut redacted = Vec::new();
    for token in value.split_whitespace() {
        redacted.push(redact_token(token));
    }
    redacted.join(" ")
}

pub(in crate::v2) fn detect_prompt_injection_pattern(value: &str) -> Option<&'static str> {
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

pub(in crate::v2) fn redact_token(token: &str) -> String {
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

pub(in crate::v2) fn current_time_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => match i64::try_from(duration.as_millis()) {
            Ok(value) => value,
            Err(_) => i64::MAX,
        },
        Err(_) => 0,
    }
}
