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

impl From<&ScreenBufferKind> for NodeScreenBufferKind {
    fn from(value: &ScreenBufferKind) -> Self {
        match value {
            ScreenBufferKind::Normal => Self::Normal,
            ScreenBufferKind::Alternate => Self::Alternate,
            ScreenBufferKind::Unknown => Self::Unknown,
        }
    }
}

impl From<&ScreenProgressState> for NodeScreenProgressState {
    fn from(value: &ScreenProgressState) -> Self {
        match value {
            ScreenProgressState::Inactive => Self::Inactive,
            ScreenProgressState::Normal => Self::Normal,
            ScreenProgressState::Error => Self::Error,
            ScreenProgressState::Indeterminate => Self::Indeterminate,
            ScreenProgressState::Warning => Self::Warning,
        }
    }
}

impl From<&ScreenProgress> for NodeScreenProgress {
    fn from(value: &ScreenProgress) -> Self {
        Self { state: (&value.state).into(), value: value.value }
    }
}

fn node_screen_buffer_kind(value: &ScreenBufferKind) -> Option<NodeScreenBufferKind> {
    if *value == ScreenBufferKind::Normal { None } else { Some(value.into()) }
}

fn node_screen_progress(value: &ScreenProgress) -> Option<NodeScreenProgress> {
    (!value.is_inactive()).then(|| value.into())
}

impl From<&ScreenCursor> for NodeScreenCursor {
    fn from(value: &ScreenCursor) -> Self {
        Self {
            row: value.row,
            col: value.col,
            shape: value.shape.as_ref().map(Into::into),
            blinking: value.blinking.then_some(true),
        }
    }
}

impl From<&ScreenCursorShape> for NodeScreenCursorShape {
    fn from(value: &ScreenCursorShape) -> Self {
        match value {
            ScreenCursorShape::Block => Self::Block,
            ScreenCursorShape::Underline => Self::Underline,
            ScreenCursorShape::Beam => Self::Beam,
            ScreenCursorShape::HollowBlock => Self::HollowBlock,
            ScreenCursorShape::Hidden => Self::Hidden,
        }
    }
}

impl From<&ScreenColor> for NodeScreenColor {
    fn from(value: &ScreenColor) -> Self {
        match value {
            ScreenColor::Named { name } => Self::Named { name: name.clone() },
            ScreenColor::Indexed { index } => Self::Indexed { index: *index },
            ScreenColor::Rgb { r, g, b } => Self::Rgb { r: *r, g: *g, b: *b },
        }
    }
}

impl From<&ScreenSurfacePalette> for NodeScreenSurfacePalette {
    fn from(value: &ScreenSurfacePalette) -> Self {
        Self {
            foreground: value.foreground.as_ref().map(Into::into),
            background: value.background.as_ref().map(Into::into),
            cursor: value.cursor.as_ref().map(Into::into),
        }
    }
}

fn node_screen_surface_palette(value: &ScreenSurfacePalette) -> Option<NodeScreenSurfacePalette> {
    (!value.is_empty()).then(|| value.into())
}

impl From<&ScreenUnderlineStyle> for NodeScreenUnderlineStyle {
    fn from(value: &ScreenUnderlineStyle) -> Self {
        match value {
            ScreenUnderlineStyle::Single => Self::Single,
            ScreenUnderlineStyle::Double => Self::Double,
            ScreenUnderlineStyle::Curly => Self::Curly,
            ScreenUnderlineStyle::Dotted => Self::Dotted,
            ScreenUnderlineStyle::Dashed => Self::Dashed,
        }
    }
}

impl From<&ScreenTextBorderStyle> for NodeScreenTextBorderStyle {
    fn from(value: &ScreenTextBorderStyle) -> Self {
        match value {
            ScreenTextBorderStyle::Framed => Self::Framed,
            ScreenTextBorderStyle::Encircled => Self::Encircled,
        }
    }
}

impl From<&ScreenTextBaseline> for NodeScreenTextBaseline {
    fn from(value: &ScreenTextBaseline) -> Self {
        match value {
            ScreenTextBaseline::Superscript => Self::Superscript,
            ScreenTextBaseline::Subscript => Self::Subscript,
        }
    }
}

impl From<&ScreenTextStyle> for NodeScreenTextStyle {
    fn from(value: &ScreenTextStyle) -> Self {
        Self {
            foreground: value.foreground.as_ref().map(Into::into),
            background: value.background.as_ref().map(Into::into),
            underline_color: value.underline_color.as_ref().map(Into::into),
            bold: value.bold,
            dim: value.dim,
            italic: value.italic,
            blink: value.blink,
            underline: value.underline.as_ref().map(Into::into),
            overline: value.overline,
            border: value.border.as_ref().map(Into::into),
            baseline: value.baseline.as_ref().map(Into::into),
            inverse: value.inverse,
            hidden: value.hidden,
            strikethrough: value.strikethrough,
            hyperlink: value.hyperlink.clone(),
        }
    }
}

impl From<&ScreenLineSpan> for NodeScreenLineSpan {
    fn from(value: &ScreenLineSpan) -> Self {
        Self { text: value.text.clone(), style: (&value.style).into() }
    }
}

impl From<&ScreenLineMediaKind> for NodeScreenLineMediaKind {
    fn from(value: &ScreenLineMediaKind) -> Self {
        match value {
            ScreenLineMediaKind::KittyGraphics => Self::KittyGraphics,
            ScreenLineMediaKind::Iterm2Image => Self::Iterm2Image,
            ScreenLineMediaKind::Sixel => Self::Sixel,
        }
    }
}

impl From<&ScreenLineMedia> for NodeScreenLineMedia {
    fn from(value: &ScreenLineMedia) -> Self {
        Self {
            kind: (&value.kind).into(),
            name: value.name.clone(),
            byte_size: value.byte_size,
            width: value.width.clone(),
            height: value.height.clone(),
            preserve_aspect_ratio: value.preserve_aspect_ratio,
            inline: value.inline.then_some(true),
            mime_type: value.mime_type.clone(),
            data_base64: value.data_base64.clone(),
            truncated: value.truncated.then_some(true),
        }
    }
}

fn node_screen_line_media(value: &[ScreenLineMedia]) -> Option<Vec<NodeScreenLineMedia>> {
    (!value.is_empty()).then(|| value.iter().map(Into::into).collect())
}

impl From<&ScreenLineSideEffectKind> for NodeScreenLineSideEffectKind {
    fn from(value: &ScreenLineSideEffectKind) -> Self {
        match value {
            ScreenLineSideEffectKind::ClipboardWrite => Self::ClipboardWrite,
            ScreenLineSideEffectKind::ClipboardRead => Self::ClipboardRead,
            ScreenLineSideEffectKind::DesktopNotification => Self::DesktopNotification,
        }
    }
}

impl From<&ScreenLineSideEffectDisposition> for NodeScreenLineSideEffectDisposition {
    fn from(value: &ScreenLineSideEffectDisposition) -> Self {
        match value {
            ScreenLineSideEffectDisposition::Blocked => Self::Blocked,
        }
    }
}

impl From<&ScreenLineSideEffectTarget> for NodeScreenLineSideEffectTarget {
    fn from(value: &ScreenLineSideEffectTarget) -> Self {
        match value {
            ScreenLineSideEffectTarget::Clipboard => Self::Clipboard,
            ScreenLineSideEffectTarget::Selection => Self::Selection,
            ScreenLineSideEffectTarget::DesktopNotification => Self::DesktopNotification,
            ScreenLineSideEffectTarget::Unknown => Self::Unknown,
        }
    }
}

impl From<&ScreenLineSideEffect> for NodeScreenLineSideEffect {
    fn from(value: &ScreenLineSideEffect) -> Self {
        Self {
            kind: (&value.kind).into(),
            disposition: (&value.disposition).into(),
            target: value.target.as_ref().map(Into::into),
            message: value.message.clone(),
        }
    }
}

fn node_screen_line_side_effects(
    value: &[ScreenLineSideEffect],
) -> Option<Vec<NodeScreenLineSideEffect>> {
    (!value.is_empty()).then(|| value.iter().map(Into::into).collect())
}

impl From<&ScreenLineSemanticMarkKind> for NodeScreenLineSemanticMarkKind {
    fn from(value: &ScreenLineSemanticMarkKind) -> Self {
        match value {
            ScreenLineSemanticMarkKind::PromptStart => Self::PromptStart,
            ScreenLineSemanticMarkKind::InputStart => Self::InputStart,
            ScreenLineSemanticMarkKind::OutputStart => Self::OutputStart,
            ScreenLineSemanticMarkKind::CommandFinished => Self::CommandFinished,
        }
    }
}

impl From<&ScreenLineSemanticMark> for NodeScreenLineSemanticMark {
    fn from(value: &ScreenLineSemanticMark) -> Self {
        Self {
            kind: (&value.kind).into(),
            col: (value.col > 0).then_some(value.col),
            exit_code: value.exit_code,
        }
    }
}

fn node_screen_line_semantic_marks(
    value: &[ScreenLineSemanticMark],
) -> Option<Vec<NodeScreenLineSemanticMark>> {
    (!value.is_empty()).then(|| value.iter().map(Into::into).collect())
}

impl From<&ScreenLine> for NodeScreenLine {
    fn from(value: &ScreenLine) -> Self {
        Self {
            text: value.text.clone(),
            spans: value.spans.iter().map(Into::into).collect(),
            media: node_screen_line_media(&value.media),
            side_effects: node_screen_line_side_effects(&value.side_effects),
            semantic_marks: node_screen_line_semantic_marks(&value.semantic_marks),
            wrapped: value.wrapped.then_some(true),
        }
    }
}

impl From<&ScreenSurface> for NodeScreenSurface {
    fn from(value: &ScreenSurface) -> Self {
        Self {
            title: value.title.clone(),
            working_directory_uri: value.working_directory_uri.clone(),
            user_variables: (!value.user_variables.is_empty())
                .then_some(value.user_variables.clone()),
            cursor: value.cursor.as_ref().map(Into::into),
            palette: node_screen_surface_palette(&value.palette),
            bell_count: node_screen_bell_count(value.bell_count),
            progress: node_screen_progress(&value.progress),
            lines: value.lines.iter().map(Into::into).collect(),
        }
    }
}

fn node_screen_bell_count(value: u64) -> Option<u64> {
    (value > 0).then_some(value)
}

impl From<&ScreenSnapshot> for NodeScreenSnapshot {
    fn from(value: &ScreenSnapshot) -> Self {
        Self {
            pane_id: value.pane_id.0.to_string(),
            sequence: value.sequence,
            rows: value.rows,
            cols: value.cols,
            source: (&value.source).into(),
            buffer_kind: node_screen_buffer_kind(&value.buffer_kind),
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
            working_directory_uri_changed: value.working_directory_uri_changed,
            working_directory_uri: value.working_directory_uri.clone(),
            user_variables_changed: value.user_variables_changed,
            user_variables: value.user_variables.clone(),
            cursor_changed: value.cursor_changed,
            cursor: value.cursor.as_ref().map(Into::into),
            palette_changed: value.palette_changed,
            palette: value.palette.as_ref().map(Into::into),
            bell_count_changed: value.bell_count_changed,
            bell_count: value.bell_count,
            progress_changed: value.progress_changed,
            progress: value.progress.as_ref().map(Into::into),
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
            buffer_kind: node_screen_buffer_kind(&value.buffer_kind),
            patch: value.patch.as_ref().map(Into::into),
            full_replace: value.full_replace.as_ref().map(Into::into),
        }
    }
}
