use super::{
    super::super::*, support_facts::collect_support_bundle_facts,
    support_manifest::build_support_bundle_manifest,
};

pub(in crate::v2) fn build_support_bundle_diagnostics(
    connection: &mut SqliteConnection,
    db_path: &Path,
    config: &TerminalPersistenceV2Config,
    bundle: &SupportBundleRow,
    now: i64,
) -> Result<SupportBundleDiagnosticsRecord, TerminalPersistenceV2Error> {
    let scope_hash = blake3_hash_text(&bundle.scope_json);
    let include_raw = bundle.include_raw != 0;
    let facts = collect_support_bundle_facts(connection, db_path, config, now)?;
    let manifest =
        build_support_bundle_manifest(db_path, bundle, now, &scope_hash, include_raw, facts);

    Ok(SupportBundleDiagnosticsRecord {
        support_bundle_id: bundle.id.clone(),
        generated_at_ms: now,
        include_raw,
        raw_content_included: include_raw,
        manifest_json: manifest,
    })
}
