/// Seconds since the Unix epoch.
pub type Instant = i64;

/// One hour, in seconds.
pub const HOUR: Instant = 3_600;

/// One day, in seconds.
pub const DAY: Instant = 86_400;

/// Seconds since the Unix epoch; zero if the system clock reads earlier than that.
pub fn seconds() -> Instant {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as Instant)
        .unwrap_or_default()
}

/// Milliseconds since the Unix epoch; zero if the system clock reads earlier than that.
pub fn millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or_default()
}
