use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_session_cursors)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct SessionCursorRow {
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) next_commit_seq: i64,
    pub(in crate::v2) writer_generation: Option<String>,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_session_cursors)]
pub(in crate::v2) struct NewSessionCursorRow {
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) next_commit_seq: i64,
    pub(in crate::v2) writer_generation: Option<String>,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_stream_cursors)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct StreamCursorRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) next_event_seq: i64,
    pub(in crate::v2) next_byte_seq: i64,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_stream_cursors)]
pub(in crate::v2) struct NewStreamCursorRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) next_event_seq: i64,
    pub(in crate::v2) next_byte_seq: i64,
    pub(in crate::v2) updated_at_ms: i64,
}
