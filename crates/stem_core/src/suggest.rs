//! "Did you mean…?" for closed namespaces.
//!
//! Lives in `stem_core` because edit distance over strings is domain-free — which
//! is this crate's entry condition — and because two closed namespaces now need
//! it: `stem_phonology`'s feature names (M1) and `stem_lexicon`'s concept keys
//! (M2). `docs/adr/0004`'s argument, that a closed set turns the worst failure
//! mode into the best one, only lands if the diagnostic actually names the near
//! miss. Without a suggestion, "unknown feature `+voicee`" is only marginally
//! better than silence.
//!
//! Hand-rolled rather than pulling in a string-similarity crate: the candidate
//! sets are closed lists of at most a few hundred short strings, so this is a
//! small need and `CLAUDE.md` says not to reach for a dependency for one.

/// The candidate nearest to `name`, or `None` if nothing is close enough.
///
/// Ties break on the candidate string, so the suggestion is deterministic — a
/// diagnostic that varies between runs is a diagnostic nobody can test, and
/// `DESIGN.md` §9.4 does not exempt error messages.
///
/// The distance ceiling is deliberately tight: suggesting `voice` for `xyzzy` is
/// worse than suggesting nothing, because it sends the reader looking for a
/// relationship that is not there.
///
/// Comparison is case-insensitive so that a lowercase `nose` finds the concept
/// key `NOSE`, but the candidate is returned in its own casing — the caller is
/// telling the user what to type, and what to type is the real spelling.
pub fn nearest<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let lowered = name.to_ascii_lowercase();
    candidates
        .map(|candidate| {
            (
                distance(&lowered, &candidate.to_ascii_lowercase()),
                candidate,
            )
        })
        .filter(|&(d, _)| d <= MAX_DISTANCE)
        .min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)))
        .map(|(_, candidate)| candidate)
}

/// How far a typo may be from its intended spelling before a suggestion becomes
/// noise. Two, because [`distance`] counts a transposition as one edit — so this
/// still admits "one slip plus one more" while staying tight enough that a short
/// key does not reach an unrelated word.
const MAX_DISTANCE: usize = 2;

/// Optimal string alignment distance: Levenshtein, plus **transposition of two
/// adjacent characters as a single edit**.
///
/// The transposition rule is not a refinement, it is the point. Swapping two
/// letters is the most common typing mistake there is, and plain Levenshtein
/// scores it as two edits — which puts a genuine near-miss into a tie with
/// unrelated words. Measured on this project's own concept list: `NOES` is
/// Levenshtein-2 from `NOSE` *and* from `NEW` and `NOT`, so the deterministic
/// tie-break returns `NEW` and the diagnostic is worse than useless. Counting the
/// swap as one edit makes `NOSE` win outright.
///
/// Three rows rather than two, since a transposition looks back two.
fn distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let width = b_chars.len() + 1;

    // `two_back` is only meaningful from the third row onward; it is never read
    // before then, because the transposition arm requires i >= 1 && j >= 1.
    let mut two_back = vec![0usize; width];
    let mut previous: Vec<usize> = (0..width).collect();
    let mut current = vec![0usize; width];

    for (i, &a_char) in a_chars.iter().enumerate() {
        current[0] = i + 1;
        for (j, &b_char) in b_chars.iter().enumerate() {
            let cost = usize::from(a_char != b_char);
            let mut best = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);

            if i > 0 && j > 0 && a_char == b_chars[j - 1] && a_chars[i - 1] == b_char {
                best = best.min(two_back[j - 1] + 1);
            }

            current[j + 1] = best;
        }
        std::mem::swap(&mut two_back, &mut previous);
        std::mem::swap(&mut previous, &mut current);
    }

    previous[b_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANDIDATES: &[&str] = &["voice", "nasal", "lateral", "coronal", "dorsal"];

    fn nearest_of(name: &str) -> Option<&'static str> {
        nearest(name, CANDIDATES.iter().copied())
    }

    #[test]
    fn a_single_letter_typo_finds_its_word() {
        assert_eq!(nearest_of("voicee"), Some("voice"));
        assert_eq!(nearest_of("nasl"), Some("nasal"));
    }

    #[test]
    fn a_distant_string_suggests_nothing() {
        assert_eq!(
            nearest_of("xyzzy"),
            None,
            "a wrong suggestion is worse than none"
        );
    }

    #[test]
    fn matching_is_case_insensitive_but_the_candidate_keeps_its_own_casing() {
        assert_eq!(nearest_of("VOICE"), Some("voice"));
    }

    /// Determinism does not stop at error messages (`DESIGN.md` §9.4).
    #[test]
    fn a_suggestion_is_deterministic_when_two_candidates_tie() {
        // "aa" is distance 1 from both "aaa" and "ab"… construct an exact tie and
        // assert the lexicographically smaller candidate wins, whichever order the
        // candidates arrive in.
        let forward = nearest("cat", ["bat", "hat"].into_iter());
        let backward = nearest("cat", ["hat", "bat"].into_iter());
        assert_eq!(forward, Some("bat"));
        assert_eq!(backward, Some("bat"), "order of candidates must not matter");
    }

    #[test]
    fn an_exact_match_returns_itself() {
        assert_eq!(nearest_of("dorsal"), Some("dorsal"));
    }

    /// The reason this is optimal string alignment rather than plain Levenshtein.
    /// Under Levenshtein a swap costs 2, which ties `noes` with `new` and `not`
    /// and makes the deterministic tie-break return the wrong word.
    #[test]
    fn a_transposition_counts_as_one_edit_so_a_swap_beats_unrelated_words() {
        assert_eq!(distance("noes", "nose"), 1);
        assert_eq!(distance("nose", "noes"), 1);
        assert_eq!(
            nearest("noes", ["nose", "new", "not"].into_iter()),
            Some("nose")
        );
    }

    #[test]
    fn distance_agrees_with_levenshtein_where_no_swap_is_involved() {
        assert_eq!(distance("", ""), 0);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("abc", ""), 3);
        assert_eq!(distance("abc", "abc"), 0);
        assert_eq!(distance("voicee", "voice"), 1);
        assert_eq!(distance("kitten", "sitting"), 3);
    }

    #[test]
    fn an_empty_candidate_list_suggests_nothing() {
        assert_eq!(nearest("voice", std::iter::empty()), None);
    }
}
