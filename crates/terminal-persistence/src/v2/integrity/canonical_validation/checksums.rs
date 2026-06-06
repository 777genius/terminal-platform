use super::super::super::*;

pub(in crate::v2) fn validate_checksum_bytes(
    row_kind: &str,
    id: &str,
    payload: &[u8],
    algorithm: &str,
    expected: &str,
    failures: &mut Vec<String>,
) {
    if algorithm != "blake3" {
        failures.push(format!("{row_kind}:{id} uses unsupported checksum algorithm {algorithm}"));
        return;
    }
    let actual = blake3_hash_bytes(payload);
    if actual != expected {
        failures.push(format!("{row_kind}:{id} checksum mismatch"));
    }
}

pub(in crate::v2) fn validate_checksum_text(
    row_kind: &str,
    id: &str,
    payload: &str,
    algorithm: &str,
    expected: &str,
    failures: &mut Vec<String>,
) {
    validate_checksum_bytes(row_kind, id, payload.as_bytes(), algorithm, expected, failures);
}

pub(in crate::v2) fn validate_payload_schema_ref(
    row_kind: &str,
    id: &str,
    payload_present: bool,
    payload_schema_id: Option<&str>,
    schema_ids: &[String],
    failures: &mut Vec<String>,
) {
    if !payload_present {
        return;
    }
    let Some(payload_schema_id) = payload_schema_id else {
        failures.push(format!("{row_kind}:{id} missing payload_schema_id"));
        return;
    };
    if !schema_ids.iter().any(|schema_id| schema_id == payload_schema_id) {
        failures.push(format!(
            "{row_kind}:{id} references unknown payload_schema_id {payload_schema_id}"
        ));
    }
}
