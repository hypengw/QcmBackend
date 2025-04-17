use chrono::{DateTime, Utc};

pub fn epoch() -> DateTime<Utc> {
    return DateTime::from_timestamp_millis(0).unwrap();
}
