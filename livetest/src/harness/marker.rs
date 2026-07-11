//! Ownership marking.
//!
//! Every object the harness creates carries a recognizable, namespaced marker
//! in a human-readable field (`description`, `name`, or a username prefix). The
//! sweep deletes *only* marker-bearing objects, so real account data without
//! the marker is never touched. The marker is matched by prefix, not by a
//! specific run token, so objects orphaned by any prior run are reclaimable.

/// Prefix stamped on every harness-created object. Matched by the sweep to
/// distinguish harness fixtures from real account data.
pub const OWNED_MARKER: &str = "livetest-";

/// Username prefix for sub-accounts, which have length/charset limits that the
/// full marker would blow. Still uniquely attributable to the harness.
pub const OWNED_USERNAME_PREFIX: &str = "lvt";

/// A per-run token, unique per process invocation, so concurrent runs and
/// crashed prior runs never collide and each orphan is attributable.
#[derive(Clone, Debug)]
pub struct RunToken(String);

impl RunToken {
    /// Derive a token from the process id and a monotonic-ish clock. Uniqueness
    /// across concurrent runs matters more than unpredictability -- these are
    /// labels, not secrets.
    pub fn new() -> Self {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        // Base36-ish compaction to keep names short for length-limited fields.
        RunToken(format!("{pid:x}{:x}", nanos & 0xffff_ffff))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A marker embeddable in a free-text field: `livetest-<token>-<seq>`.
    pub fn marker(&self, seq: u32) -> String {
        format!("{OWNED_MARKER}{}-{seq}", self.0)
    }

    /// A marker for `name` fields with a tight length limit (e.g. a ring
    /// group's, which rejects the full [`marker`](Self::marker) with
    /// `name_toolong`). Keeps the `livetest-` prefix so [`is_owned_marker`]
    /// still reclaims it, trailing the last 4 token chars plus `seq` for
    /// same-run distinctness -- at most `9 + 4 + 1 + len(seq)` chars.
    pub fn short_marker(&self, seq: u32) -> String {
        let token = &self.0;
        let tail = &token[token.len().saturating_sub(4)..];
        format!("{OWNED_MARKER}{tail}{seq}")
    }

    /// A sub-account username: `lvt<token><seq>`, kept short and alphanumeric.
    pub fn username(&self, seq: u32) -> String {
        format!("{OWNED_USERNAME_PREFIX}{}{seq}", self.0)
    }
}

impl Default for RunToken {
    fn default() -> Self {
        Self::new()
    }
}

/// True when `value` looks like a harness-created marker from *any* run.
pub fn is_owned_marker(value: &str) -> bool {
    value.contains(OWNED_MARKER)
}

/// True when `username` looks like a harness-created sub-account from any run.
pub fn is_owned_username(username: &str) -> bool {
    username.starts_with(OWNED_USERNAME_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_marker_is_reclaimable_and_shorter_than_the_full_marker() {
        let token = RunToken("1869f12345678".into());
        let short = token.short_marker(2);
        // Still recognized by the sweep.
        assert!(
            is_owned_marker(&short),
            "short marker must carry the prefix"
        );
        // Materially shorter than the full marker that triggers `name_toolong`.
        assert!(short.len() < token.marker(2).len());
        // Distinct per seq so same-run fixtures don't collide.
        assert_ne!(token.short_marker(0), token.short_marker(1));
    }

    #[test]
    fn short_marker_handles_a_token_shorter_than_the_tail_window() {
        // `saturating_sub` must not panic on a sub-4-char token.
        let short = RunToken("ab".into()).short_marker(0);
        assert!(is_owned_marker(&short));
    }
}
