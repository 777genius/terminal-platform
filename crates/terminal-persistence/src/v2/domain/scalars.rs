use super::super::*;

pub(in crate::v2) fn validate_positive_dimensions(
    rows: i32,
    cols: i32,
) -> Result<(), TerminalPersistenceV2Error> {
    if rows <= 0 || cols <= 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "terminal dimensions must be positive, got rows={rows}, cols={cols}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn validate_optional_range(
    low: Option<i64>,
    high: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    match (low, high) {
        (Some(low), Some(high)) if low <= high => Ok(()),
        (None, None) => Ok(()),
        _ => Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} range must be either empty or fully populated"
        ))),
    }
}

pub(in crate::v2) fn validate_optional_half_open_range(
    low: Option<i64>,
    high: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    match (low, high) {
        (Some(low), Some(high)) if low < high => Ok(()),
        (None, None) => Ok(()),
        _ => Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} range must be empty or half-open with low < high"
        ))),
    }
}

pub(in crate::v2) fn validate_non_negative_seq(
    value: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    if let Some(value) = value
        && value < 0
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} must not be negative"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn checked_len(
    len: usize,
    label: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    i64::try_from(len).map_err(|_| {
        TerminalPersistenceV2Error::InvalidData(format!("{label} does not fit in i64"))
    })
}

pub(in crate::v2) fn u64_to_i64(
    value: u64,
    label: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    i64::try_from(value).map_err(|_| {
        TerminalPersistenceV2Error::InvalidData(format!("{label} does not fit in i64"))
    })
}
