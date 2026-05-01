use super::{prelude::*, *};

impl From<&SubscriptionId> for NodeSubscriptionMeta {
    fn from(value: &SubscriptionId) -> Self {
        Self { subscription_id: value.0.to_string() }
    }
}

impl From<&SubscriptionEvent> for NodeSubscriptionEvent {
    fn from(value: &SubscriptionEvent) -> Self {
        match value {
            SubscriptionEvent::TopologySnapshot(snapshot) => {
                Self::TopologySnapshot(snapshot.into())
            }
            SubscriptionEvent::ScreenDelta(delta) => Self::ScreenDelta(delta.into()),
            SubscriptionEvent::SessionHealthSnapshot(snapshot) => {
                Self::SessionHealthSnapshot(snapshot.into())
            }
        }
    }
}

impl From<&ProjectionSource> for NodeProjectionSource {
    fn from(value: &ProjectionSource) -> Self {
        match value {
            ProjectionSource::NativeEmulator => Self::NativeEmulator,
            ProjectionSource::NativeTranscript => Self::NativeTranscript,
            ProjectionSource::TmuxCapturePane => Self::TmuxCapturePane,
            ProjectionSource::TmuxRawOutputImport => Self::TmuxRawOutputImport,
            ProjectionSource::ZellijViewportSubscribe => Self::ZellijViewportSubscribe,
            ProjectionSource::ZellijDumpSnapshot => Self::ZellijDumpSnapshot,
        }
    }
}

impl From<&ScreenCursor> for NodeScreenCursor {
    fn from(value: &ScreenCursor) -> Self {
        Self { row: value.row, col: value.col }
    }
}

impl From<&ScreenLine> for NodeScreenLine {
    fn from(value: &ScreenLine) -> Self {
        Self { text: value.text.clone() }
    }
}

impl From<&ScreenSurface> for NodeScreenSurface {
    fn from(value: &ScreenSurface) -> Self {
        Self {
            title: value.title.clone(),
            cursor: value.cursor.as_ref().map(Into::into),
            lines: value.lines.iter().map(Into::into).collect(),
        }
    }
}

impl From<&ScreenSnapshot> for NodeScreenSnapshot {
    fn from(value: &ScreenSnapshot) -> Self {
        Self {
            pane_id: value.pane_id.0.to_string(),
            sequence: value.sequence,
            rows: value.rows,
            cols: value.cols,
            source: (&value.source).into(),
            surface: (&value.surface).into(),
        }
    }
}

impl From<&ScreenLinePatch> for NodeScreenLinePatch {
    fn from(value: &ScreenLinePatch) -> Self {
        Self { row: value.row, line: (&value.line).into() }
    }
}

impl From<&ScreenPatch> for NodeScreenPatch {
    fn from(value: &ScreenPatch) -> Self {
        Self {
            title_changed: value.title_changed,
            title: value.title.clone(),
            cursor_changed: value.cursor_changed,
            cursor: value.cursor.as_ref().map(Into::into),
            line_updates: value.line_updates.iter().map(Into::into).collect(),
        }
    }
}

impl From<&ScreenDelta> for NodeScreenDelta {
    fn from(value: &ScreenDelta) -> Self {
        Self {
            pane_id: value.pane_id.0.to_string(),
            from_sequence: value.from_sequence,
            to_sequence: value.to_sequence,
            rows: value.rows,
            cols: value.cols,
            source: (&value.source).into(),
            patch: value.patch.as_ref().map(Into::into),
            full_replace: value.full_replace.as_ref().map(Into::into),
        }
    }
}
