//! System sources for repository identifiers and time.
//!
//! These are the default concrete adapters for the [`UuidSource`] and [`Clock`]
//! ports. They wrap operating-system randomness and the system clock. Tests
//! inject deterministic sources instead.

use crate::domain::ports::{Clock, UuidSource};

/// Generates repository identifiers with UUID v4 from the operating-system RNG.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemUuidSource;

impl UuidSource for SystemUuidSource {
    fn new_repository_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

/// Produces the current UTC timestamp in RFC 3339 format.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc_rfc3339(&self) -> String {
        use time::format_description::well_known::Rfc3339;
        time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }
}
