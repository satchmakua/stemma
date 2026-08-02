//! The recorded causal history of one word (`DESIGN.md` §3.3, §11.2).
//!
//! # Why this lives in `stem_lexicon` and not in `stem_soundchange`
//!
//! `stem_soundchange` already depends on `stem_lexicon` (it takes a `&Lexicon`),
//! so a `Derivation` defined there and stored on a `WordEntry` would be a crate
//! cycle that does not compile. It does not need to be there: a trace names
//! `RuleId` and `PhonemeId` (`stem_core`) and `Root` (`stem_phonology`), and
//! nothing else — it never mentions `SoundChangeRule`. Putting it here is the
//! same shape as `Root` living in `stem_phonology` while `stem_lexicon` stores
//! one, and it has a second payoff: M5's export can render a derivation without
//! depending on the rule engine at all.

use serde::{Deserialize, Serialize};
use stem_core::{PhonemeId, RuleId};
use stem_phonology::Root;

/// One word's derivation.
///
/// # Intermediate forms are not stored
///
/// Only `input` plus per-site deltas. [`Self::replay`] reconstructs every
/// intermediate. A stored snapshot beside the edits would be a second source of
/// truth that desynchronises the first time anything touches either — the same
/// argument that keeps `form` off [`crate::WordEntry`] (`docs/adr/0007`). It also
/// means the file grows with *edits* rather than with forms, and that replay is
/// load-bearing: if it were wrong there would be no intermediate forms at all, so
/// it cannot silently decay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derivation {
    /// The form entering the **first** rule this word ever met. A second
    /// `apply-rules` run extends `steps` and leaves this alone, so a derivation
    /// always begins at the proto-form however many strata were applied.
    pub input: Root,
    /// One entry per rule that touched this word, in application order.
    ///
    /// A rule that matched nothing produces no entry. That is not a gap: "which
    /// rules did this word see?" is answered by the genome's `applied_rules`, and
    /// a rule present there and absent here **did not apply** — which is what lets
    /// `stemma trace` print that line. Storing it per word would store a derivable
    /// value.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<RuleApplication>,
}

/// One rule's effect on one word.
///
/// Emitted when `!sites.is_empty() || !blocked.is_empty()`. The second disjunct
/// matters: a rule that matched everywhere and was refused everywhere would
/// otherwise vanish, and "the rule was refused" is precisely the fact the user
/// needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleApplication {
    /// Which rule.
    pub rule: RuleId,
    /// This rule's index in the genome's `applied_rules`, **globally** — offset by
    /// however many rules had already been applied when this run started. M4 needs
    /// "the daughter diverges after rule 7", and a per-run index cannot say that
    /// once a lineage has two strata.
    pub index: u32,
    /// One entry **per application site**, leftmost first. Not one per rule per
    /// word: a rule that voices two consonants produced two events, and collapsing
    /// them loses the spans that make M5's view legible.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<SiteTrace>,
    /// Sites that matched and were then refused, with why. "Why didn't my rule
    /// apply here?" is the single most common question in every sound-change
    /// tool's troubleshooting threads and is unanswerable without this.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked: Vec<BlockedSite>,
}

/// One application site.
///
/// All coordinates are **snapshot coordinates** — indices into the form as it
/// stood before this rule. After a deletion they no longer index the rule's
/// output; [`Derivation::replay`] and the renderer both address the input form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteTrace {
    /// Flat index into the form as it stood before this rule. §11.2 writes
    /// `"span": [2, 3]` over a string; a form here is a segment vector, so the
    /// span is `[at, at + 1)` over `Root::segments()` order and the view renders
    /// it as such.
    pub at: u32,
    /// The segment that stood at `at`.
    pub before: PhonemeId,
    /// `None` when the segment was deleted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<PhonemeId>,
    /// How the output bundle got its symbol. `None` on a deletion.
    ///
    /// Inside the trace, not beside it: "this rule applied, and it created a new
    /// phoneme /ɡ/" is causal history, and burying phonemic innovation in a
    /// rendering helper would be exactly the correct-but-silent bug this milestone
    /// exists to forbid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<SymbolResolution>,
    /// The matched context, **outward from the target**: `left[0]` is the segment
    /// immediately left. `None` is a word edge. §11.2's `"environment": "a _ a"`
    /// is this, rendered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<Option<PhonemeId>>,
    /// The matched right context, outward from the target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub right: Vec<Option<PhonemeId>>,
    /// The stale pattern of a syllable this site emptied and removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emptied_syllable: Option<String>,
}

/// How a post-change bundle acquired a symbol.
///
/// **Evaluated against the inventory as the run received it**, never against the
/// growing one. `Inventory` therefore means "the author declared this segment" and
/// `Innovated` means "the author did not" — a property of the segment, not of
/// where its word sits in the lexicon. Without that pin, moving one word above
/// another in a fixture would flip two traces between `Inventory` and `Innovated`
/// without changing a single output form, and a derivation computed for one word
/// alone would disagree with the same word computed inside its lexicon — which
/// would break the compositionality M5's per-word recomputation needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SymbolResolution {
    /// The language already declared this segment.
    Inventory {
        /// The first declared phoneme with the output bundle, in authored order.
        phoneme: PhonemeId,
        /// Other declared phonemes with the identical bundle, in authored order.
        /// Non-empty only where `phonology.identical_features` already warns.
        /// Recorded rather than hidden: first-declared-wins with no diagnostic is
        /// Lexurgy issue #9, and the fix is to *say so*.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        ambiguous_with: Vec<PhonemeId>,
    },
    /// The reference table had it, and the language has **gained** a phoneme.
    /// Phonemic split — the normal engine of phonological history, not an error.
    Innovated {
        /// The minted phoneme's id, `ph_{slug}` off the table row.
        phoneme: PhonemeId,
        /// The minted glyph, for reading the trace without the inventory to hand.
        ipa: String,
    },
}

/// A site that matched and was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockedSite {
    /// Flat snapshot index of the matched target.
    pub at: u32,
    /// Why the site did not apply.
    pub reason: BlockReason,
}

/// Why a matched site was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockReason {
    /// No declared phoneme and no reference row has this bundle. Carries
    /// `FeatureBundle::render()` in full, so the user can read exactly what the
    /// engine could not name and add the row.
    Unnameable {
        /// The rendered bundle.
        bundle: String,
    },
    /// The output bundle fails `required_features_missing`. Names the features.
    IllFormed {
        /// The missing feature names, in frozen order.
        missing: Vec<String>,
    },
    /// The reference row's glyph is already held by a different bundle in this
    /// language. Minting would make the inventory invalid (`duplicate_ipa` is an
    /// Error), so the site is refused instead.
    SymbolHeld {
        /// The glyph in contention.
        ipa: String,
        /// The phoneme already holding it.
        by: PhonemeId,
    },
    /// `ph_{slug}` is taken by a phoneme with a different bundle.
    IdHeld {
        /// The contended id.
        id: PhonemeId,
    },
    /// A `Copy` whose donor values no articulator of the node.
    DonorHasNoNode {
        /// The node that could not be copied.
        node: String,
        /// The donor segment.
        donor: PhonemeId,
    },
    /// Applying every deletion this rule matched would leave the word with no
    /// segments. Set-level, so the whole rule's deletion set for this word is
    /// refused — see the application contract in `stem_soundchange::apply`.
    WouldEmptyWord,
}

impl Derivation {
    /// Every intermediate form, one per step, folding `steps` over `input`.
    ///
    /// Consults no rule, no inventory and no engine — it applies stored
    /// [`PhonemeId`]s at stored indices, including dropping a syllable that
    /// empties. So §16.3's property ("every trace output's final form equals the
    /// stored word form") is a real statement about the *file*: the trace is
    /// sufficient to reconstruct the form, i.e. not lossy.
    ///
    /// It is deliberately **not** an independent oracle for whether the engine
    /// computed the right form — replay and apply share one edit discipline, and
    /// pretending otherwise would be a check that cannot fail. The independent
    /// oracle is the hand-computed golden in `fixtures/`.
    pub fn replay(&self) -> Vec<Root> {
        let mut current = self.input.clone();
        let mut forms = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            // Commit descending by snapshot index, exactly as the engine does, so
            // pending indices do not shift under earlier edits.
            let mut sites: Vec<&SiteTrace> = step.sites.iter().collect();
            sites.sort_by_key(|s| std::cmp::Reverse(s.at));
            for site in sites {
                match &site.after {
                    Some(id) => {
                        current.replace_at(site.at as usize, id.clone());
                    }
                    None => {
                        current.remove_at(site.at as usize);
                    }
                }
            }
            forms.push(current.clone());
        }
        forms
    }

    /// The last intermediate, or `input` when `steps` is empty.
    pub fn final_form(&self) -> Root {
        self.replay().pop().unwrap_or_else(|| self.input.clone())
    }

    /// The surface segments descending from the input positions in `[start, end)`,
    /// in surface order (M8, `M8-SPEC` §6).
    ///
    /// This is how a *morpheme's* surface allomorph is recovered: a `MorphemeRef`
    /// records the flat span the morpheme occupied in the composition form — which
    /// **is** [`Self::input`], because a later stratum extends `steps` and never
    /// touches `input` — and this returns what that span became after every rule.
    /// So `tira-`**`ka`** (suffix span `[4, 6)`) surfaces as `[ph_g, ph_a]` once
    /// intervocalic voicing has fired, and `tan-`**`ka`** (span `[3, 5)`) surfaces
    /// as `[ph_k, ph_a]` because no rule touched it — the two allomorphs of one
    /// suffix, read straight off the trace.
    ///
    /// Mirrors [`Self::replay`]'s edit discipline exactly (descending commit per
    /// step) while carrying a parallel `Vec` of each surviving segment's **origin**
    /// input-index: a `Replace` keeps its position's origin, a `Delete` drops it.
    /// Consulting only the stored trace — no rule, no inventory, no engine — it is
    /// exact and general across strata, and returns the raw input span when `steps`
    /// is empty. Out-of-range site indices are skipped rather than panicking; a
    /// well-formed trace never produces one, so this only guards a corrupt file.
    pub fn surface_of_input_span(&self, start: usize, end: usize) -> Vec<PhonemeId> {
        // Each entry is (current segment, origin index into `input`). The origin is
        // fixed at composition time and rides along through every edit, so a
        // survivor can always be traced back to the morpheme it came from.
        let mut current: Vec<(PhonemeId, usize)> = self
            .input
            .segments()
            .cloned()
            .enumerate()
            .map(|(origin, seg)| (seg, origin))
            .collect();

        for step in &self.steps {
            let mut sites: Vec<&SiteTrace> = step.sites.iter().collect();
            // Descending, exactly as `replay`, so a deletion cannot shift the index
            // of a site not yet committed within this step.
            sites.sort_by_key(|s| std::cmp::Reverse(s.at));
            for site in sites {
                let at = site.at as usize;
                if at >= current.len() {
                    continue; // corrupt trace; a valid one never indexes past the form
                }
                match &site.after {
                    // A replacement keeps the position — and thus its origin.
                    Some(id) => current[at].0 = id.clone(),
                    // A deletion removes the segment and its origin together.
                    None => {
                        current.remove(at);
                    }
                }
            }
        }

        current
            .into_iter()
            .filter(|(_, origin)| *origin >= start && *origin < end)
            .map(|(seg, _)| seg)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_phonology::Syllable;

    fn syllable(pattern: &str, segments: &[&str]) -> Syllable {
        Syllable {
            pattern: pattern.to_owned(),
            segments: segments.iter().map(|s| PhonemeId::new(*s)).collect(),
            stress: None,
        }
    }

    fn takala_like() -> Root {
        Root {
            syllables: vec![
                syllable("CV", &["ph_t", "ph_a"]),
                syllable("CV", &["ph_k", "ph_a"]),
                syllable("CV", &["ph_l", "ph_a"]),
            ],
        }
    }

    #[test]
    fn replay_folds_replacements_and_deletions_over_the_input() {
        let derivation = Derivation {
            input: takala_like(),
            steps: vec![
                RuleApplication {
                    rule: RuleId::new("r_0001"),
                    index: 0,
                    sites: vec![SiteTrace {
                        at: 2,
                        before: PhonemeId::new("ph_k"),
                        after: Some(PhonemeId::new("ph_g")),
                        resolution: None,
                        left: vec![Some(PhonemeId::new("ph_a"))],
                        right: vec![Some(PhonemeId::new("ph_a"))],
                        emptied_syllable: None,
                    }],
                    blocked: vec![],
                },
                RuleApplication {
                    rule: RuleId::new("r_0003"),
                    index: 2,
                    sites: vec![SiteTrace {
                        at: 5,
                        before: PhonemeId::new("ph_a"),
                        after: None,
                        resolution: None,
                        left: vec![Some(PhonemeId::new("ph_l"))],
                        right: vec![None],
                        emptied_syllable: None,
                    }],
                    blocked: vec![],
                },
            ],
        };

        let forms = derivation.replay();
        assert_eq!(forms.len(), 2);

        let first: Vec<&str> = forms[0].segments().map(|s| s.as_str()).collect();
        assert_eq!(first, ["ph_t", "ph_a", "ph_g", "ph_a", "ph_l", "ph_a"]);

        let second: Vec<&str> = forms[1].segments().map(|s| s.as_str()).collect();
        assert_eq!(second, ["ph_t", "ph_a", "ph_g", "ph_a", "ph_l"]);
        assert_eq!(
            forms[1].syllables.len(),
            3,
            "the syllable shrank but did not empty, so it survives"
        );
        assert_eq!(derivation.final_form(), forms[1]);
    }

    #[test]
    fn replay_drops_a_syllable_its_last_deletion_empties() {
        let input = Root {
            syllables: vec![syllable("CV", &["ph_t", "ph_a"]), syllable("V", &["ph_a"])],
        };
        let derivation = Derivation {
            input,
            steps: vec![RuleApplication {
                rule: RuleId::new("r_0003"),
                index: 0,
                sites: vec![SiteTrace {
                    at: 2,
                    before: PhonemeId::new("ph_a"),
                    after: None,
                    resolution: None,
                    left: vec![Some(PhonemeId::new("ph_a"))],
                    right: vec![None],
                    emptied_syllable: Some("V".to_owned()),
                }],
                blocked: vec![],
            }],
        };
        let final_form = derivation.final_form();
        assert_eq!(
            final_form.syllables.len(),
            1,
            "the emptied syllable is gone"
        );
        assert_eq!(final_form.len(), 2);
    }

    #[test]
    fn an_empty_derivation_replays_to_its_input() {
        let derivation = Derivation {
            input: takala_like(),
            steps: vec![],
        };
        assert!(derivation.replay().is_empty());
        assert_eq!(derivation.final_form(), takala_like());
    }

    /// The M8 span reader over a replacement: `ta.ka.la`'s middle `/k/` voices to
    /// `/ɡ/`, and the span `[2, 4)` — the second syllable — must surface as
    /// `[ph_g, ph_a]`, its `/k/` carried through the edit as `/ɡ/`.
    #[test]
    fn surface_of_input_span_follows_a_replacement_through_its_origin() {
        let derivation = Derivation {
            input: takala_like(),
            steps: vec![RuleApplication {
                rule: RuleId::new("r_ivv"),
                index: 0,
                sites: vec![SiteTrace {
                    at: 2,
                    before: PhonemeId::new("ph_k"),
                    after: Some(PhonemeId::new("ph_g")),
                    resolution: None,
                    left: vec![Some(PhonemeId::new("ph_a"))],
                    right: vec![Some(PhonemeId::new("ph_a"))],
                    emptied_syllable: None,
                }],
                blocked: vec![],
            }],
        };
        let surface = derivation.surface_of_input_span(2, 4);
        let span: Vec<&str> = surface.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            span,
            ["ph_g", "ph_a"],
            "the /k/ surfaces as /ɡ/ in its span"
        );
    }

    /// A deletion inside the span drops that segment from the recovered allomorph,
    /// and a deletion *before* the span does not shift what the span recovers —
    /// origins are stable under the edit, unlike flat indices.
    #[test]
    fn surface_of_input_span_drops_a_deleted_segment_and_is_stable_across_a_prior_deletion() {
        // `ta.ka.la`: delete index 1 (`a` of `ta`) and index 5 (final `a`).
        let derivation = Derivation {
            input: takala_like(),
            steps: vec![RuleApplication {
                rule: RuleId::new("r_del"),
                index: 0,
                sites: vec![
                    SiteTrace {
                        at: 1,
                        before: PhonemeId::new("ph_a"),
                        after: None,
                        resolution: None,
                        left: vec![],
                        right: vec![],
                        emptied_syllable: None,
                    },
                    SiteTrace {
                        at: 5,
                        before: PhonemeId::new("ph_a"),
                        after: None,
                        resolution: None,
                        left: vec![],
                        right: vec![],
                        emptied_syllable: None,
                    },
                ],
                blocked: vec![],
            }],
        };
        // The third syllable `la` occupies origin span [4, 6); its final `a` (origin
        // 5) is deleted, so only `l` survives — and the earlier deletion at 1 did
        // not perturb that, because the span is addressed by origin, not by index.
        let surface = derivation.surface_of_input_span(4, 6);
        let span: Vec<&str> = surface.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            span,
            ["ph_l"],
            "the deleted final vowel is gone from the span"
        );
    }

    /// With no steps, the span reader is a pure slice of the input — the regular,
    /// pre-sound-change allomorph.
    #[test]
    fn surface_of_input_span_of_an_untouched_form_is_the_raw_input_slice() {
        let derivation = Derivation {
            input: takala_like(),
            steps: vec![],
        };
        let surface = derivation.surface_of_input_span(2, 4);
        let span: Vec<&str> = surface.iter().map(|s| s.as_str()).collect();
        assert_eq!(span, ["ph_k", "ph_a"]);
    }

    #[test]
    fn multiple_sites_in_one_step_commit_descending_so_indices_do_not_shift() {
        // Two deletions in one rule: at 1 and at 3. Committing ascending would
        // shift index 3 to 2 before its own edit; descending is order-safe.
        let input = Root {
            syllables: vec![
                syllable("CV", &["ph_t", "ph_a"]),
                syllable("CV", &["ph_k", "ph_a"]),
            ],
        };
        let derivation = Derivation {
            input,
            steps: vec![RuleApplication {
                rule: RuleId::new("r_x"),
                index: 0,
                sites: vec![
                    SiteTrace {
                        at: 1,
                        before: PhonemeId::new("ph_a"),
                        after: None,
                        resolution: None,
                        left: vec![],
                        right: vec![],
                        emptied_syllable: None,
                    },
                    SiteTrace {
                        at: 3,
                        before: PhonemeId::new("ph_a"),
                        after: None,
                        resolution: None,
                        left: vec![],
                        right: vec![],
                        emptied_syllable: None,
                    },
                ],
                blocked: vec![],
            }],
        };
        let final_form = derivation.final_form();
        let segments: Vec<&str> = final_form.segments().map(|s| s.as_str()).collect();
        assert_eq!(segments, ["ph_t", "ph_k"]);
    }

    #[test]
    fn a_derivation_round_trips_through_ron() {
        let derivation = Derivation {
            input: takala_like(),
            steps: vec![RuleApplication {
                rule: RuleId::new("r_0001"),
                index: 0,
                sites: vec![SiteTrace {
                    at: 2,
                    before: PhonemeId::new("ph_k"),
                    after: Some(PhonemeId::new("ph_g")),
                    resolution: Some(SymbolResolution::Innovated {
                        phoneme: PhonemeId::new("ph_g"),
                        ipa: "\u{0261}".to_owned(),
                    }),
                    left: vec![Some(PhonemeId::new("ph_a"))],
                    right: vec![Some(PhonemeId::new("ph_a"))],
                    emptied_syllable: None,
                }],
                blocked: vec![BlockedSite {
                    at: 4,
                    reason: BlockReason::Unnameable {
                        bundle: "+syllabic -voice".to_owned(),
                    },
                }],
            }],
        };
        let text = ron::ser::to_string(&derivation).expect("serialise");
        let back: Derivation = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, derivation);
    }

    #[test]
    fn a_misspelled_trace_field_fails_to_load_rather_than_defaulting() {
        let text = r#"(
            input: (syllables: [(pattern: "CV", segments: ["ph_t", "ph_a"])]),
            stepps: [],
        )"#;
        assert!(ron::from_str::<Derivation>(text).is_err());
    }
}
