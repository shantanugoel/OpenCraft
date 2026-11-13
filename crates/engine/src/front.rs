//! The front application, as the shell reports it (decision 0035).
//!
//! The shell sends each notice. The engine holds the policy.

/// The front application, as the shell reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontApplication {
    /// An application with this bundle identifier, for example `com.apple.Safari`.
    Bundle(String),
    /// An application with no bundle identifier.
    NoBundle,
    /// The process of the shell itself.
    Shell,
}

/// Why the active profile is active.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActiveCause {
    /// A manual selection, or the start.
    #[default]
    Selected,
    /// The front application has this link to the profile.
    Linked { bundle_id: String },
    /// The front application has no link, and this is the default profile.
    Default,
}

/// The most links one profile can have (decision 0035, section 4).
pub const MAX_LINKS: usize = 32;

/// True if `id` is a bundle identifier: 1 to 255 characters, ASCII letters,
/// digits, `-`, and `.`.
pub fn valid_bundle_id(id: &str) -> bool {
    (1..=255).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
}

/// True if the two bundle identifiers are the same with no difference
/// between small and capital letters (ASCII).
pub fn same_bundle(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}
