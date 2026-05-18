//! Cheat search — value-finder for the cheats system.
//!
//! Workflow:
//!   1. User opens the "Cheat search" panel + clicks Start. We snapshot
//!      the current SystemRam (or chosen region) into the session.
//!   2. User does something in-game that should change the value
//!      they're looking for (e.g. lose a life).
//!   3. User picks a filter (changed / unchanged / increased / decreased
//!      / equal_to_value(N)). Each filter compares the CURRENT
//!      memory_snapshot against the LAST snapshot stored in the session,
//!      narrows the candidate offset list, then refreshes the session's
//!      snapshot for the next filter.
//!   4. After a few filters the candidate list is small enough to act
//!      on. Each candidate has a "Make cheat" button in the UI that
//!      pre-fills the cheat editor.
//!
//! v1 scope:
//!   - Width is 1 byte. Multi-byte (2/4 LE) widths are a follow-up;
//!     users can manually expand a found 1-byte hit to 2 or 4 in the
//!     cheat editor.
//!   - Only the four numeric-comparison filters above + an
//!     equal-to-value lookup. "Difference of N", "Greater than N",
//!     "Less than N" are tractable follow-ups using the same predicate
//!     dispatch shape.

use serde::{Deserialize, Serialize};

/// State kept across the lifetime of one search session. Lives in
/// `AppState.cheat_search` as `Mutex<Option<CheatSearchSession>>` — the
/// option boundary is "no search in progress".
#[derive(Debug, Clone)]
pub struct CheatSearchSession {
    /// Memory region tag (`MemoryRegionId::as_str()`).
    pub region: String,
    /// 1 byte today. Carried for forward compat with multi-byte search.
    pub width: u8,
    /// The bytes that were at each offset on the LAST filter pass
    /// (or initial Start). Compared against the live `memory_snapshot`
    /// to evaluate predicates.
    pub previous: Vec<u8>,
    /// Sorted byte offsets that still match every filter applied so far.
    /// Initialized to every aligned offset on Start; pruned by each
    /// filter call.
    pub candidates: Vec<u32>,
}

/// Filters the UI exposes. Wire format is the `kind` discriminant —
/// `value` is only inspected for `EqualToValue`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum CheatSearchFilter {
    /// Match offsets whose byte changed since the last snapshot.
    Changed,
    /// Match offsets whose byte stayed identical.
    Unchanged,
    /// Match offsets whose byte strictly increased.
    Increased,
    /// Match offsets whose byte strictly decreased.
    Decreased,
    /// Match offsets whose current byte equals this exact value (0..=255).
    /// Useful when the user knows the target value (e.g. life count = 3).
    EqualToValue(i64),
}

impl CheatSearchFilter {
    /// Return true if `(prev, curr)` satisfies the filter. Width-1
    /// predicate — multi-byte filters compose by AND-ing across widths.
    pub fn matches(self, prev: u8, curr: u8) -> bool {
        match self {
            Self::Changed => prev != curr,
            Self::Unchanged => prev == curr,
            Self::Increased => curr > prev,
            Self::Decreased => curr < prev,
            Self::EqualToValue(v) => curr as i64 == v,
        }
    }
}

/// Filter the candidate list in-place. After the call:
///   - `candidates` contains only offsets where the predicate matched
///   - `previous` is updated to mirror `current` so the next filter
///     compares against the new baseline
pub fn apply_filter(
    session: &mut CheatSearchSession,
    current: &[u8],
    filter: CheatSearchFilter,
) {
    session.candidates.retain(|&offset| {
        let i = offset as usize;
        if i >= session.previous.len() || i >= current.len() {
            return false;
        }
        filter.matches(session.previous[i], current[i])
    });
    // Refresh the baseline so the next filter compares against the
    // post-filter values. Cheap to clone — region snapshots are usually
    // 2-128 KB.
    session.previous.clear();
    session.previous.extend_from_slice(current);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheatSearchResult {
    pub offset: u32,
    pub current_value: u8,
    pub previous_value: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheatSearchSummary {
    pub region: String,
    pub width: u8,
    pub candidate_count: usize,
    /// Up to `limit` candidates with current + previous values, for the
    /// UI's candidate list. The full list lives in the session.
    pub top: Vec<CheatSearchResult>,
}

pub fn summarize(session: &CheatSearchSession, current: &[u8], limit: usize) -> CheatSearchSummary {
    let top: Vec<CheatSearchResult> = session
        .candidates
        .iter()
        .take(limit)
        .map(|&offset| {
            let i = offset as usize;
            CheatSearchResult {
                offset,
                current_value: *current.get(i).unwrap_or(&0),
                previous_value: *session.previous.get(i).unwrap_or(&0),
            }
        })
        .collect();
    CheatSearchSummary {
        region: session.region.clone(),
        width: session.width,
        candidate_count: session.candidates.len(),
        top,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(prev: &[u8]) -> CheatSearchSession {
        CheatSearchSession {
            region: "system_ram".into(),
            width: 1,
            previous: prev.to_vec(),
            candidates: (0..prev.len() as u32).collect(),
        }
    }

    #[test]
    fn filter_changed_narrows_to_diff_indices() {
        let prev = [10u8, 20, 30, 40];
        let curr = [10u8, 21, 30, 99];
        let mut s = session(&prev);
        apply_filter(&mut s, &curr, CheatSearchFilter::Changed);
        assert_eq!(s.candidates, vec![1, 3]);
        // baseline refreshed
        assert_eq!(s.previous, curr);
    }

    #[test]
    fn filter_unchanged_narrows_to_same_indices() {
        let prev = [5u8, 5, 5, 5];
        let curr = [5u8, 5, 6, 5];
        let mut s = session(&prev);
        apply_filter(&mut s, &curr, CheatSearchFilter::Unchanged);
        assert_eq!(s.candidates, vec![0, 1, 3]);
    }

    #[test]
    fn filter_increased() {
        let prev = [3u8, 5, 8];
        let curr = [4u8, 5, 1];
        let mut s = session(&prev);
        apply_filter(&mut s, &curr, CheatSearchFilter::Increased);
        assert_eq!(s.candidates, vec![0]);
    }

    #[test]
    fn filter_decreased() {
        let prev = [3u8, 5, 8];
        let curr = [4u8, 5, 1];
        let mut s = session(&prev);
        apply_filter(&mut s, &curr, CheatSearchFilter::Decreased);
        assert_eq!(s.candidates, vec![2]);
    }

    #[test]
    fn filter_equal_to_value() {
        let prev = [0u8; 4];
        let curr = [7u8, 0, 7, 7];
        let mut s = session(&prev);
        apply_filter(&mut s, &curr, CheatSearchFilter::EqualToValue(7));
        assert_eq!(s.candidates, vec![0, 2, 3]);
    }

    #[test]
    fn chained_filters_narrow_further() {
        // Simulate finding a "life count" value:
        //   start with 3 (initial)
        //   take a hit (decreased to 2) — Decreased filter
        //   gain a life (increased to 3) — Increased filter
        // Only one address should survive both filters.
        let mut s = session(&[3u8, 3, 99, 3]);
        apply_filter(&mut s, &[2u8, 3, 99, 2], CheatSearchFilter::Decreased);
        // After: candidates = [0, 3] (both went 3→2)
        apply_filter(&mut s, &[3u8, 3, 99, 0], CheatSearchFilter::Increased);
        // After: candidates = [0] only (3 stayed 2→0, decreased not increased)
        assert_eq!(s.candidates, vec![0]);
    }

    #[test]
    fn summary_caps_top_at_limit() {
        let mut s = session(&[0u8; 100]);
        // All 100 candidates "changed" (curr all 1s).
        let curr = vec![1u8; 100];
        apply_filter(&mut s, &curr, CheatSearchFilter::Changed);
        let summary = summarize(&s, &curr, 10);
        assert_eq!(summary.candidate_count, 100);
        assert_eq!(summary.top.len(), 10);
        assert_eq!(summary.top[0].offset, 0);
        assert_eq!(summary.top[0].current_value, 1);
    }
}
