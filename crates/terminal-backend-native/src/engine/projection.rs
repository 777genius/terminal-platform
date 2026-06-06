use terminal_backend_api::BackendError;
use terminal_projection::{ProjectionSource, ScreenDelta, ScreenSnapshot};

use super::{
    SNAPSHOT_HISTORY_LIMIT,
    model::{NativePaneRuntime, PaneGeometry},
};

impl NativePaneRuntime {
    pub(super) fn mark_surface_dirty(&self) {
        super::signals::bump_watch(&self.surface_tick);
    }

    pub(super) fn render_snapshot(
        &self,
        title: Option<String>,
    ) -> Result<ScreenSnapshot, BackendError> {
        let geometry = self.geometry()?;
        let rows = geometry.rows;
        let cols = geometry.cols;
        let rendered = self.emulator.render(title.clone());
        let mut projection = self
            .projection
            .lock()
            .map_err(|_| BackendError::internal("native pane projection state lock poisoned"))?;

        if let Some(current) = projection.history.back()
            && current.rows == rows
            && current.cols == cols
            && current.source == ProjectionSource::NativeEmulator
            && current.surface == rendered.surface
        {
            return Ok(current.clone());
        }

        let sequence = projection.history.back().map_or(1, |snapshot| snapshot.sequence + 1);
        let snapshot = ScreenSnapshot {
            pane_id: self.pane_id,
            sequence,
            rows,
            cols,
            source: ProjectionSource::NativeEmulator,
            surface: rendered.surface,
        };

        projection.history.push_back(snapshot.clone());
        while projection.history.len() > SNAPSHOT_HISTORY_LIMIT {
            projection.history.pop_front();
        }

        Ok(snapshot)
    }

    pub(super) fn screen_delta(
        &self,
        title: Option<String>,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let current = self.render_snapshot(title)?;
        if current.sequence == from_sequence {
            return Ok(ScreenDelta::unchanged_from(&current));
        }

        let projection = self
            .projection
            .lock()
            .map_err(|_| BackendError::internal("native pane projection state lock poisoned"))?;
        let previous =
            projection.history.iter().find(|snapshot| snapshot.sequence == from_sequence);

        Ok(match previous {
            Some(previous) => ScreenDelta::between(previous, &current),
            None => ScreenDelta::full_replace(from_sequence, &current),
        })
    }

    pub(super) fn geometry(&self) -> Result<PaneGeometry, BackendError> {
        self.geometry
            .lock()
            .map(|geometry| *geometry)
            .map_err(|_| BackendError::internal("native pane geometry lock poisoned"))
    }
}
