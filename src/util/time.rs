use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::local::error::FsError;

pub fn time_to_float(time: &SystemTime) -> Result<f64, FsError> {
    let timestamp = time
        .duration_since(UNIX_EPOCH)
        .map_err(|e| FsError::TimeError(e.to_string()))?;
    Ok(timestamp.as_secs_f64())
}

pub fn float_to_time(timestamp: f64) -> Result<SystemTime, FsError> {
    let duration = Duration::from_secs_f64(timestamp);
    let time = UNIX_EPOCH.checked_add(duration);
    time.ok_or_else(|| FsError::TimeError("Error adding time".to_string()))
}
