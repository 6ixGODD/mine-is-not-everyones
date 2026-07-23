//! Side-effect ports used by application services.
//!
//! The port traits live in the domain so that application logic depends on
//! abstractions rather than concrete system sources. Concrete adapters (for
//! example [`crate::infrastructure::system`]) implement these ports. Keeping
//! the ports here lets the initialization service stay deterministic and
//! unit-testable with injected sources of identifiers and time.

/// Source of fresh repository identifiers (UUID v4).
///
/// The default system implementation generates a random UUID. Tests inject a
/// fixed source so repository-identity resolution is deterministic.
pub trait UuidSource {
    /// Returns a new repository identifier string.
    fn new_repository_id(&self) -> String;
}

/// Source of the current UTC timestamp in RFC 3339 format.
///
/// The default system implementation reads the operating-system clock. Tests
/// inject a fixed clock so newly created design markers are deterministic.
pub trait Clock {
    /// Returns the current UTC time as an RFC 3339 string.
    fn now_utc_rfc3339(&self) -> String;
}
