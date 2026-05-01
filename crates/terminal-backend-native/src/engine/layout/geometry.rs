use super::super::{DEFAULT_SPLIT_RATIO_BPS, SPLIT_RATIO_SCALE};

pub(super) fn target_to_first_span(total: u16, desired_target: u16, target_is_first: bool) -> u16 {
    if total <= 1 {
        return 1;
    }

    let clamped_target = desired_target.clamp(1, total.saturating_sub(1));
    if target_is_first {
        clamped_target
    } else {
        total.saturating_sub(clamped_target).clamp(1, total.saturating_sub(1))
    }
}

pub(super) fn span_to_ratio_bps(first_span: u16, total: u16) -> u16 {
    if total <= 1 {
        return DEFAULT_SPLIT_RATIO_BPS;
    }

    let clamped_first = first_span.clamp(1, total.saturating_sub(1));
    let ratio = ((u32::from(clamped_first) * u32::from(SPLIT_RATIO_SCALE))
        + (u32::from(total) / 2))
        / u32::from(total);
    ratio
        .clamp(1, u32::from(SPLIT_RATIO_SCALE.saturating_sub(1)))
        .try_into()
        .unwrap_or(DEFAULT_SPLIT_RATIO_BPS)
}

pub(super) fn partition_dimension_by_ratio(total: u16, ratio_bps: u16) -> (u16, u16) {
    if total <= 1 {
        return (1, 1);
    }

    let ratio = ratio_bps.clamp(1, SPLIT_RATIO_SCALE.saturating_sub(1));
    let mut first = ((u32::from(total) * u32::from(ratio)) + (u32::from(SPLIT_RATIO_SCALE) / 2))
        / u32::from(SPLIT_RATIO_SCALE);
    first = first.clamp(1, u32::from(total.saturating_sub(1)));
    let first: u16 = first.try_into().unwrap_or(1);
    let second = total.saturating_sub(first).max(1);
    (first, second)
}
