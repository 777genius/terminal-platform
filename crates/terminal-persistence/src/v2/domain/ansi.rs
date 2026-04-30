pub(in crate::v2) fn event_scope(session_id: &str, pane_id: Option<&str>) -> EventScope {
    match pane_id {
        Some(pane_id) => EventScope { kind: "pane".to_string(), id: pane_id.to_string() },
        None => EventScope { kind: "session".to_string(), id: session_id.to_string() },
    }
}

pub(in crate::v2) struct EventScope {
    pub(in crate::v2) kind: String,
    pub(in crate::v2) id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::v2) struct BufferModeTransition {
    pub(in crate::v2) action: &'static str,
    pub(in crate::v2) target_buffer_kind: &'static str,
    pub(in crate::v2) mode: i32,
    pub(in crate::v2) byte_offset: i64,
    pub(in crate::v2) byte_len: i64,
}

pub(in crate::v2) fn detect_buffer_mode_transitions(payload: &[u8]) -> Vec<BufferModeTransition> {
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

pub(in crate::v2) fn is_csi_final_byte(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

pub(in crate::v2) fn parse_private_mode_params(params: &[u8]) -> Vec<i32> {
    params
        .split(|byte| matches!(*byte, b';' | b':'))
        .filter_map(|part| std::str::from_utf8(part).ok()?.parse::<i32>().ok())
        .collect()
}
