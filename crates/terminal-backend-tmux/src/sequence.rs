use crate::prelude::*;

pub(crate) fn screen_sequence(
    pane_id: PaneId,
    rows: u16,
    cols: u16,
    title: Option<&str>,
    lines: &[ScreenLine],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    pane_id.hash(&mut hasher);
    rows.hash(&mut hasher);
    cols.hash(&mut hasher);
    title.hash(&mut hasher);
    for line in lines {
        line.text.hash(&mut hasher);
    }
    hasher.finish()
}
