//! The reference symbol table: how a feature bundle gets an IPA string.
//!
//! When a rule produces a bundle no inventory phoneme carries — voicing /k/ in a
//! language with no /ɡ/ — the engine must *name* the result before it can store,
//! render, or export it. This table is where the name comes from.
//!
//! # Why a flat compiled-in table and not a diacritic composer
//!
//! Composition has no canonical modifier order, so equal bundles would produce
//! unequal strings — fatal under §9.4. Lexurgy issue #22 (a doubled stress mark,
//! closed *wontfix*) is composition failing to canonicalise in the most mature
//! tool in the space, and its commit 8105f80 ("moved normalization after
//! segmentation") is the Unicode half of the same bug. A powerset search over *n*
//! diacritics is 2^n; ASCA has thirty of them and falls back to U+FFFD.
//!
//! Stemma does not have those problems because it does not have that feature. The
//! feature set is a closed enum of sixteen (`docs/adr/0004`) and inventories are
//! fifteen to forty segments, so a twenty-row table dominates on every axis.
//!
//! # Why twenty rows and not five thousand
//!
//! The table is a function **from Stemma's sixteen features**, not from the IPA
//! chart. /t/, /t̪/ and /ʈ/ are one bundle here, so there is one row and it says
//! `t`. Adding /ʃ/ would be a *bug*: without a stridency or anteriority feature it
//! is byte-identical to /s/, and a non-injective table is Lexurgy issue #9 — a
//! silent first-declared-wins substitution. `no_two_reference_rows_share_a_
//! feature_bundle` is what keeps that unrepresentable.
//!
//! Growth rule: add a row when a rule needs it, one reviewed line at a time.

use std::sync::OnceLock;

use crate::features::FeatureBundle;
use crate::phoneme::SegmentKind;

/// One segment the engine can name.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceSegment {
    /// The IPA string, NFC, written as `\u{…}` escapes in source where the glyph
    /// is confusable, so no editor can normalise it out from under us.
    pub ipa: &'static str,
    /// ASCII slug. A minted phoneme's id is `ph_{slug}`. Unique across the table.
    pub slug: &'static str,
    /// A romanisation, set **only** where the IPA glyph is a Unicode-identity
    /// variant of the ASCII letter a language would obviously use for it. In this
    /// table that is exactly one row: `ɡ` U+0261 romanises as `g`. Everything else
    /// is `None`, because an invented romanisation is worse than an honest IPA
    /// character — writing /ŋ/ as `ng` would make it indistinguishable from
    /// /n/+/ɡ/ in the rendered form the homophone check counts.
    pub romanization: Option<&'static str>,
    /// The phonotactic slot. **The table is the authority**; nothing is inherited
    /// from the input segment, because a second source of truth for `kind` would
    /// make the minted phoneme's content depend on which word reached it first.
    pub kind: SegmentKind,
    /// The bundle, as signed names, parsed once at first use.
    pub features: &'static [&'static str],
}

/// The table, in a frozen authored order.
///
/// Mints are appended to an inventory in **this** order, so the evolved inventory
/// is a function of the *set* of innovations rather than of lexicon traversal.
pub static REFERENCE_SEGMENTS: &[ReferenceSegment] = &[
    ReferenceSegment {
        ipa: "b",
        slug: "b",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "-continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "+labial",
            "-coronal",
            "-dorsal",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "d",
        slug: "d",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "-continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "+coronal",
            "-dorsal",
        ],
    },
    // U+0261 LATIN SMALL LETTER SCRIPT G — not ASCII `g` U+0067. Distinct code
    // points, NFC/NFD-unrelated, near-identical in most fonts, which is what makes
    // confusing them silent. The romanisation is where the ASCII letter belongs.
    ReferenceSegment {
        ipa: "\u{0261}",
        slug: "g",
        romanization: Some("g"),
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "-continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "+back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{0294}", // ʔ
        slug: "glottal_stop",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "-continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "-voice",
            "-labial",
            "-coronal",
            "-dorsal",
        ],
    },
    ReferenceSegment {
        ipa: "\u{0278}", // ɸ
        slug: "phi",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "-voice",
            "+labial",
            "-coronal",
            "-dorsal",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{03B2}", // β
        slug: "beta",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "+labial",
            "-coronal",
            "-dorsal",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "z",
        slug: "z",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "+coronal",
            "-dorsal",
        ],
    },
    ReferenceSegment {
        ipa: "x",
        slug: "x",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "-voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "+back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{0263}", // ɣ
        slug: "gamma",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "+back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{00E7}", // ç — NFC precomposed, deliberately not c + combining cedilla
        slug: "c_cedilla",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "-sonorant",
            "-approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "-voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "-back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{014B}", // ŋ
        slug: "eng",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "+sonorant",
            "-approximant",
            "-continuant",
            "+nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "+back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{0272}", // ɲ
        slug: "enye",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "+sonorant",
            "-approximant",
            "-continuant",
            "+nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "-back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{0279}", // ɹ
        slug: "turned_r",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "+sonorant",
            "+approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "+coronal",
            "-dorsal",
        ],
    },
    ReferenceSegment {
        ipa: "\u{028E}", // ʎ
        slug: "turned_y",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "+consonantal",
            "+sonorant",
            "+approximant",
            "+continuant",
            "-nasal",
            "+lateral",
            "-trill",
            "+voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "-back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{0270}", // ɰ
        slug: "turned_m_leg",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "-consonantal",
            "+sonorant",
            "+approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "+back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{0265}", // ɥ
        slug: "turned_h",
        romanization: None,
        kind: SegmentKind::Consonant,
        features: &[
            "-syllabic",
            "-consonantal",
            "+sonorant",
            "+approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "+labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "-back",
            "+round",
        ],
    },
    ReferenceSegment {
        ipa: "y",
        slug: "y",
        romanization: None,
        kind: SegmentKind::Vowel,
        features: &[
            "+syllabic",
            "-consonantal",
            "+sonorant",
            "+approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "+labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "-back",
            "+round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{00F8}", // ø — NFC precomposed
        slug: "o_slash",
        romanization: None,
        kind: SegmentKind::Vowel,
        features: &[
            "+syllabic",
            "-consonantal",
            "+sonorant",
            "+approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "+labial",
            "-coronal",
            "+dorsal",
            "-high",
            "-low",
            "-back",
            "+round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{00E6}", // æ
        slug: "ae",
        romanization: None,
        kind: SegmentKind::Vowel,
        features: &[
            "+syllabic",
            "-consonantal",
            "+sonorant",
            "+approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "-high",
            "+low",
            "-back",
            "-round",
        ],
    },
    ReferenceSegment {
        ipa: "\u{026F}", // ɯ
        slug: "turned_m",
        romanization: None,
        kind: SegmentKind::Vowel,
        features: &[
            "+syllabic",
            "-consonantal",
            "+sonorant",
            "+approximant",
            "+continuant",
            "-nasal",
            "-lateral",
            "-trill",
            "+voice",
            "-labial",
            "-coronal",
            "+dorsal",
            "+high",
            "-low",
            "+back",
            "-round",
        ],
    },
];

/// The parsed bundles, built once. `OnceLock` over a `Vec` in table order — never
/// a map, because the parsed table participates in mint ordering, which reaches
/// output.
fn parsed() -> &'static Vec<(FeatureBundle, &'static ReferenceSegment)> {
    static PARSED: OnceLock<Vec<(FeatureBundle, &'static ReferenceSegment)>> = OnceLock::new();
    PARSED.get_or_init(|| {
        REFERENCE_SEGMENTS
            .iter()
            .map(|row| {
                let bundle = FeatureBundle::try_from(
                    row.features
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect::<Vec<_>>(),
                )
                .unwrap_or_else(|e| panic!("reference row `{}` does not parse: {e}", row.slug));
                (bundle, row)
            })
            .collect()
    })
}

/// Exact-bundle lookup. `None` rather than an approximation, at every call site,
/// forever.
///
/// With sixteen ternary features the representable space is ~43 million and the
/// inhabited fraction is ~5×10⁻⁷, so a near miss is near-certainly a *different*
/// phoneme. The decisive number is local, not general: on `proto_asterian.ron`,
/// `/k/` with `[+voice]` sits at Hamming distance **1 from /k/ itself** and ≥4
/// from everything else — so a nearest-neighbour resolver would return the input
/// segment, silently undoing the rule while the trace asserts it applied. That is
/// the one failure this project cannot tolerate.
///
/// Linear over twenty rows; never a `HashMap` reaching output.
pub fn lookup(bundle: FeatureBundle) -> Option<&'static ReferenceSegment> {
    parsed()
        .iter()
        .find(|(row_bundle, _)| *row_bundle == bundle)
        .map(|(_, row)| *row)
}

/// The bundle of a table row, parsed. Used by resolution and by the tests.
pub fn bundle_of(row: &'static ReferenceSegment) -> FeatureBundle {
    parsed()
        .iter()
        .find(|(_, r)| std::ptr::eq(*r, row))
        .map(|(b, _)| *b)
        .expect("every row is in the parsed table")
}

/// The position of a row in [`REFERENCE_SEGMENTS`], for mint ordering.
pub fn row_index(row: &'static ReferenceSegment) -> usize {
    REFERENCE_SEGMENTS
        .iter()
        .position(|r| std::ptr::eq(r, row))
        .expect("every row is in the table")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::required_features_missing;
    use std::collections::BTreeSet;

    /// The injectivity guarantee Lexurgy issue #9 lacked: the map is a *function*
    /// in the direction the engine uses it. This is also why /ʃ/, /θ/ and /ð/ are
    /// absent — without a stridency or anteriority feature they are byte-identical
    /// to /s/ or /z/, and adding one of them would be a bug this test catches.
    #[test]
    fn no_two_reference_rows_share_a_feature_bundle() {
        let rows = parsed();
        for (i, (a, row_a)) in rows.iter().enumerate() {
            for (b, row_b) in &rows[i + 1..] {
                assert_ne!(
                    a, b,
                    "rows `{}` and `{}` are featurally identical; the table must be injective",
                    row_a.slug, row_b.slug
                );
            }
        }
    }

    #[test]
    fn no_two_reference_rows_share_a_slug_or_an_ipa_string() {
        let slugs: BTreeSet<&str> = REFERENCE_SEGMENTS.iter().map(|r| r.slug).collect();
        let ipas: BTreeSet<&str> = REFERENCE_SEGMENTS.iter().map(|r| r.ipa).collect();
        assert_eq!(slugs.len(), REFERENCE_SEGMENTS.len(), "duplicate slug");
        assert_eq!(ipas.len(), REFERENCE_SEGMENTS.len(), "duplicate ipa");
    }

    /// Every row is a legal phoneme by the validator's own rules — the same
    /// function, so the table and the validator cannot disagree.
    #[test]
    fn every_reference_row_satisfies_the_required_feature_tables() {
        for (bundle, row) in parsed() {
            let missing = required_features_missing(*bundle);
            assert!(
                missing.is_empty(),
                "row `{}` is missing {:?}",
                row.slug,
                missing.iter().map(|f| f.name()).collect::<Vec<_>>()
            );
        }
    }

    /// The table is the only source of IPA the engine produces, and nothing
    /// normalises at runtime — so the whole normalisation bug class (Lexurgy
    /// 8105f80, panphon's asymmetric seg_dict) must be structurally absent.
    ///
    /// Every row is a single code point that is not a combining mark, which for
    /// one-scalar strings is exactly "already NFC": NFC could only change a string
    /// by composing a base with a combining character, and there is none here.
    #[test]
    fn every_reference_ipa_string_is_already_nfc() {
        for row in REFERENCE_SEGMENTS {
            let chars: Vec<char> = row.ipa.chars().collect();
            assert_eq!(chars.len(), 1, "row `{}` is not a single scalar", row.slug);
            let c = chars[0] as u32;
            assert!(
                !(0x0300..=0x036F).contains(&c),
                "row `{}` is a bare combining mark",
                row.slug
            );
        }
    }

    #[test]
    fn the_script_g_row_is_u0261_not_ascii() {
        let g = REFERENCE_SEGMENTS
            .iter()
            .find(|r| r.slug == "g")
            .expect("g row");
        assert_eq!(g.ipa, "\u{0261}");
        assert_ne!(
            g.ipa, "g",
            "ASCII g U+0067 is the romanisation, not the IPA"
        );
        assert_eq!(g.romanization, Some("g"));
    }

    #[test]
    fn lookup_finds_a_row_by_exact_bundle_and_misses_near_ones() {
        let (g_bundle, g_row) = parsed()
            .iter()
            .find(|(_, r)| r.slug == "g")
            .map(|(b, r)| (*b, *r))
            .expect("g row");
        assert!(std::ptr::eq(lookup(g_bundle).expect("exact hit"), g_row));

        // One cell off — the devoiced version is /k/'s bundle, which is not in the
        // table (the table never duplicates a fixture phoneme), so lookup misses.
        let near = g_bundle.with(crate::Feature::Voice, crate::Sign::Minus);
        assert!(lookup(near).is_none(), "a near miss must not resolve");
    }

    #[test]
    fn row_index_and_bundle_of_agree_with_the_table() {
        for (i, row) in REFERENCE_SEGMENTS.iter().enumerate() {
            assert_eq!(row_index(row), i);
            assert_eq!(lookup(bundle_of(row)).map(|r| r.slug), Some(row.slug));
        }
    }
}
