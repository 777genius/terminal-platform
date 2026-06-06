use terminal_mux_domain::SplitDirection;

use crate::engine::model::NativePaneLayoutSplit;

use super::super::geometry::partition_dimension_by_ratio;

impl NativePaneLayoutSplit {
    pub(in crate::engine::layout) fn partition(
        &self,
        rows: u16,
        cols: u16,
    ) -> ((u16, u16), (u16, u16)) {
        match self.direction {
            SplitDirection::Vertical => {
                let (first_cols, second_cols) = partition_dimension_by_ratio(cols, self.ratio_bps);
                ((rows, first_cols), (rows, second_cols))
            }
            SplitDirection::Horizontal => {
                let (first_rows, second_rows) = partition_dimension_by_ratio(rows, self.ratio_bps);
                ((first_rows, cols), (second_rows, cols))
            }
        }
    }
}
