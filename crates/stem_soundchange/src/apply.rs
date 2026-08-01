//! Ordered application — THE APPLICATION CONTRACT.
//!
//! Normative, in the register of `stem_phonology::generate`'s draw-order
//! contract. Every clause has a test named after it. **Changing any clause
//! invalidates every previously derived form.**
//!
//! # Across rules: strictly sequential
//!
//! Rules apply in order, each rule's complete output feeding the next. No
//! re-running, no fixpoint over the sequence, no reordering. `chronology_years`
//! is a label, never a sort key.
//!
//! # Across words: word-major, in `Lexicon::iter()` order
//!
//! `for word { assign stress; for rule { … } }`. Word-major rather than
//! rule-major because it is compositional: `apply_rules` over one word is
//! identical to `apply_rules` over the lexicon restricted to that word, which is
//! what M5's per-word recomputation needs. Word order is **not observable** in
//! the output — resolution is computed against the frozen input inventory and
//! mints are appended in reference-table order after the run — and a test
//! asserts it.
//!
//! # Within one rule: simultaneous over a frozen snapshot
//!
//! Three phases against a snapshot of the word **as it stood before this rule**:
//!
//! 1. **Scan.** Walk flat indices ascending. Test the target's features with
//!    `subsumes`, the target's stress against the *enclosing syllable's* mark,
//!    then the environment. Every read is from the snapshot.
//! 2. **Resolve.** Compute each matched site's output segment. A site that
//!    cannot produce a well-formed, nameable segment becomes a `BlockedSite`.
//!    Then apply the set-level `WouldEmptyWord` predicate to the surviving
//!    deletions.
//! 3. **Commit.** Apply every surviving edit in one pass, **descending by
//!    index**, so pending indices do not shift.
//!
//! **The environment is never evaluated against a partially-updated form, and a
//! `Copy`'s donor is read from the snapshot too.** The second half is not
//! implied by the first: a copy reading a live buffer makes the result depend on
//! commit direction. Snapshot donor, pinned.
//!
//! A rule's output is never visible to its own matching: `aaa` under
//! `[+syllabic] → [-syllabic] / [+syllabic] _` changes positions 1 *and* 2,
//! because both see an `a` to their left in the snapshot. Iterative
//! resume-at-replacement semantics (Brassica's default) are rejected — they let
//! a rule feed itself, admit non-termination, and make results depend on a scan
//! direction the author never stated.
//!
//! # Determinism is by construction
//!
//! At M3 a rule's target is exactly one segment, so the site set is a set of
//! distinct flat indices, each edit touches only its own index, and every
//! precondition was read from an immutable snapshot. Application is commutative
//! over the site set and scan direction is unobservable. This is why there is no
//! `ltr`/`rtl` flag and no overlap resolver: with single-segment targets they
//! have nothing to disambiguate, and an unobservable setting written into a
//! user's file is a lie about what the format means.
//!
//! When multi-segment targets arrive, the tie-breakers are pre-committed
//! (`M3-SPEC` §5.5, Lexurgy's three, verbatim): longest match wins at one start;
//! earlier-declared expression wins across expressions; leftmost wins otherwise.
//!
//! # No fast paths, deliberately
//!
//! Matching resolves every id through the resolver's working view and tests
//! `subsumes`. There is no cached class set and no "is this id authored"
//! shortcut: a membership cache keyed on the authored inventory would silently
//! exclude segments minted by earlier rules, and a segment later rules cannot
//! see is a broken feeding order. The fixture proves it directly — its lenition
//! rule targets a class that is *empty* in the proto-language, so its only
//! possible input is a segment the voicing rule minted.
//!
//! # The coordinate system
//!
//! Matching runs over the flat segment sequence, ignoring syllable boundaries;
//! the /k/ of `ta.ka.la` is intervocalic across two boundaries. The one
//! exception is the stress axis, which reads the enclosing syllable's mark. A
//! window slot past the word edge never matches; only an explicit
//! `EnvItem::Boundary` matches the edge. All `SiteTrace` coordinates are
//! snapshot coordinates.

use stem_core::{PhonemeId, Result, RuleId, StemmaError, Validate, ValidationReport};
use stem_lexicon::{
    BlockReason, BlockedSite, Derivation, Lexicon, RuleApplication, SiteTrace, SymbolResolution,
    WordEntry,
};
use stem_phonology::prosody::{Prosody, Stress};
use stem_phonology::{PhonemeInventory, Root};

use crate::resolve::Resolver;
use crate::rule::{Change, EnvItem, Environment, Position, SoundChangeRule};

/// The result of running an ordered rule sequence over a lexicon.
///
/// **The three fields are one value and must be applied together.** Taking
/// `lexicon` while keeping the original inventory yields
/// `lexicon.unknown_phoneme` — an Error — on every innovated segment.
/// `stem_genome::LanguageGenome::evolve` is the sanctioned caller and does it
/// correctly; a second caller must too.
#[derive(Debug, Clone, PartialEq)]
pub struct Evolution {
    /// The input inventory, unchanged, followed by every innovated phoneme in
    /// reference-table order.
    pub inventory: PhonemeInventory,
    /// The evolved entries in input order. `id`, `concept`, `glosses`,
    /// `part_of_speech`, `source` and **`cognate_set`** are copied verbatim:
    /// evolution changes forms, never identities (§8.6). Only `phonemic_form`
    /// and `trace` move.
    pub lexicon: Lexicon,
    /// Innovations, refusals, and rules that never applied (§16.5). Bare codes;
    /// the caller absorbs them under a scope.
    pub report: ValidationReport,
}

/// Applies `rules` in order to every entry of `lexicon`.
///
/// Takes the parts, not a genome: `stem_soundchange` sits below `stem_genome`,
/// and this is the shape `RootGenerator::new` and `build_proto_lexicon` already
/// take, for the same reason (`docs/adr/0002`).
///
/// `first_index` is the index that `rules[0]` will occupy in the genome's
/// `applied_rules`, i.e. how many rules had already been applied. It is what
/// makes `RuleApplication::index` globally meaningful across a multi-stratum
/// lineage; passing 0 for a first run is correct.
///
/// **Refuses a language whose inventory has any Error, on the unfiltered
/// report.** There is no sibling to `generation_blocking` and there must not be:
/// a private filter list is how the validator and the engine drifted apart last
/// time. Everything short of an Error proceeds and is reported (§17).
///
/// **No RNG.** A pure function of its five arguments.
pub fn apply_rules(
    rules: &[SoundChangeRule],
    first_index: u32,
    inventory: &PhonemeInventory,
    prosody: &Prosody,
    lexicon: &Lexicon,
) -> Result<Evolution> {
    let inventory_report = inventory.validate();
    if !inventory_report.is_ok() {
        return Err(StemmaError::Invalid(
            "this language's inventory cannot support a rule engine".to_owned(),
            inventory_report,
        ));
    }

    let mut resolver = Resolver::new(inventory);
    let mut report = ValidationReport::new();
    let mut evolved: Vec<WordEntry> = Vec::with_capacity(lexicon.len());
    let mut applications_per_rule = vec![0usize; rules.len()];
    // §9.5 `ambiguous_target_symbol`, once per (rule, chosen phoneme) per run —
    // every further site is the same story, and the per-site record lives in the
    // trace's `ambiguous_with` regardless. A `Vec` because the warning order must
    // follow the deterministic word-major scan, and the set stays tiny.
    let mut ambiguous_warned: Vec<(RuleId, PhonemeId)> = Vec::new();

    for entry in lexicon.iter() {
        let mut word = entry.phonemic_form.clone();
        // Once in a word's life: a word that already carries marks is left alone,
        // so splitting one rule sequence across two runs gives the same language
        // as running it once.
        prosody.assign(&mut word);

        // A second run extends an existing derivation — `input` stays the
        // proto-form, steps append, indices continue from `first_index`.
        let mut derivation = entry.trace.clone().unwrap_or_else(|| Derivation {
            input: word.clone(),
            steps: Vec::new(),
        });

        for (offset, rule) in rules.iter().enumerate() {
            let application = apply_one_rule(
                rule,
                first_index + offset as u32,
                &mut word,
                &mut resolver,
                &mut report,
                &mut ambiguous_warned,
                entry,
            );
            if let Some(application) = application {
                applications_per_rule[offset] += application.sites.len();
                derivation.steps.push(application);
            }
        }

        let mut new_entry = entry.clone();
        new_entry.phonemic_form = word;
        new_entry.trace = Some(derivation);
        evolved.push(new_entry);
    }

    // §16.5: a rule that never applied is worth saying. Conditioned on
    // applications, not matches — a rule that matched fifty sites and was refused
    // at all fifty still reports.
    for (offset, rule) in rules.iter().enumerate() {
        if applications_per_rule[offset] == 0 {
            report.note(
                "rule_never_applied",
                format!(
                    "rule `{}` (\"{}\") applied to no word in this lexicon",
                    rule.id, rule.name
                ),
            );
        }
    }

    let (final_inventory, minted) = resolver.into_inventory();
    for phoneme in &minted {
        report.note(
            "phoneme_innovated",
            format!(
                "the language gained /{}/ (`{}`) — phonemic split, the normal engine of \
                 phonological history",
                phoneme.ipa, phoneme.id
            ),
        );
    }

    let evolved_lexicon = Lexicon::from_entries(evolved);

    // A minted phoneme later rules wrote out of every word again (risk 8's /ɡ/
    // after lenition). Scoped to mints: the *authored* inventory legitimately
    // carries segments the lexicon happens not to use.
    for phoneme in &minted {
        let attested = evolved_lexicon
            .iter()
            .any(|e| e.phonemic_form.segments().any(|id| id == &phoneme.id));
        if !attested {
            report.note(
                "phoneme_now_unattested",
                format!(
                    "/{}/ (`{}`) was innovated and then written out of every word by a \
                     later rule; it remains in the inventory so earlier trace steps stay \
                     renderable",
                    phoneme.ipa, phoneme.id
                ),
            );
        }
    }

    Ok(Evolution {
        inventory: final_inventory,
        lexicon: evolved_lexicon,
        report,
    })
}

/// One pending, resolved edit.
enum Edit {
    Replace {
        at: usize,
        with: stem_core::PhonemeId,
        resolution: stem_lexicon::SymbolResolution,
        left: Vec<Option<stem_core::PhonemeId>>,
        right: Vec<Option<stem_core::PhonemeId>>,
    },
    Delete {
        at: usize,
        left: Vec<Option<stem_core::PhonemeId>>,
        right: Vec<Option<stem_core::PhonemeId>>,
    },
}

impl Edit {
    fn at(&self) -> usize {
        match self {
            Self::Replace { at, .. } | Self::Delete { at, .. } => *at,
        }
    }
}

/// Applies one rule to one word, in place. Returns the trace step, or `None`
/// when the rule neither applied nor blocked anywhere in this word.
#[allow(clippy::too_many_arguments)] // the run state `apply_rules` threads through
fn apply_one_rule(
    rule: &SoundChangeRule,
    global_index: u32,
    word: &mut Root,
    resolver: &mut Resolver<'_>,
    report: &mut ValidationReport,
    ambiguous_warned: &mut Vec<(RuleId, PhonemeId)>,
    entry: &WordEntry,
) -> Option<RuleApplication> {
    let snapshot = word.clone();
    let length = snapshot.len();

    let mut edits: Vec<Edit> = Vec::new();
    let mut blocked: Vec<BlockedSite> = Vec::new();

    for flat in 0..length {
        let Some(segment_id) = snapshot.segment_at(flat) else {
            continue;
        };
        let Some(segment) = resolver.phoneme(segment_id).cloned() else {
            // An id outside the inventory is `lexicon.unknown_phoneme`, an Error
            // the caller's validation reports; the engine skips rather than
            // guessing.
            continue;
        };

        // Target: features, then the enclosing syllable's stress.
        if !segment.features.subsumes(rule.target.features) {
            continue;
        }
        if let Some(required) = rule.target.stress {
            // `Some(Unstressed)` matches only a syllable explicitly marked
            // unstressed. An unmarked syllable — a language with no prosody —
            // does not match: absence is not minus, one tier up.
            if snapshot.syllable_at(flat).and_then(|s| s.stress) != Some(required) {
                continue;
            }
        }

        if !environment_matches(&rule.environment, &snapshot, flat, resolver) {
            continue;
        }

        // The matched context, recorded outward. At least one slot per side even
        // for an empty window, so §11.2's `environment` line is always renderable.
        let left = context_of(&snapshot, flat, rule.environment.before.len().max(1), true);
        let right = context_of(&snapshot, flat, rule.environment.after.len().max(1), false);

        match &rule.change {
            Change::Delete => edits.push(Edit::Delete {
                at: flat,
                left,
                right,
            }),
            Change::Set(delta) => {
                let out = segment.features.overlay(*delta);
                match resolver.resolve(out, segment.frequency_weight) {
                    Ok(resolution) => {
                        note_ambiguity(report, ambiguous_warned, rule, &resolution, resolver);
                        edits.push(Edit::Replace {
                            at: flat,
                            with: resolution_id(&resolution),
                            resolution,
                            left,
                            right,
                        });
                    }
                    Err(reason) => {
                        note_block(report, rule, entry, flat, &reason);
                        blocked.push(BlockedSite {
                            at: flat as u32,
                            reason,
                        });
                    }
                }
            }
            Change::Copy { from, node } => {
                // The donor index is a pure function of the target index — fixed
                // offsets, resolved before any mutation. Read from the snapshot:
                // a live read would make the result depend on commit direction.
                let donor_index = position_index(*from, flat);
                let Some(donor_id) = donor_index.and_then(|i| snapshot.segment_at(i)) else {
                    // Unreachable when the environment matched — the load-time
                    // check guarantees the position is inside the window — but
                    // never panic on data.
                    continue;
                };
                let Some(donor) = resolver.phoneme(donor_id).cloned() else {
                    continue;
                };
                match segment.features.copy_node(donor.features, *node) {
                    None => {
                        let reason = BlockReason::DonorHasNoNode {
                            node: node.name().to_owned(),
                            donor: donor_id.clone(),
                        };
                        note_block(report, rule, entry, flat, &reason);
                        blocked.push(BlockedSite {
                            at: flat as u32,
                            reason,
                        });
                    }
                    Some(out) => match resolver.resolve(out, segment.frequency_weight) {
                        Ok(resolution) => {
                            note_ambiguity(report, ambiguous_warned, rule, &resolution, resolver);
                            edits.push(Edit::Replace {
                                at: flat,
                                with: resolution_id(&resolution),
                                resolution,
                                left,
                                right,
                            });
                        }
                        Err(reason) => {
                            note_block(report, rule, entry, flat, &reason);
                            blocked.push(BlockedSite {
                                at: flat as u32,
                                reason,
                            });
                        }
                    },
                }
            }
        }
    }

    // The set-level empty-word predicate. Per-site checking is wrong twice:
    // against the snapshot it lets `a.a` under `V → ∅` delete both segments;
    // against the live word it makes the surviving segment depend on scan
    // direction, destroying commutativity. Set-level preserves both.
    let deletions = edits
        .iter()
        .filter(|e| matches!(e, Edit::Delete { .. }))
        .count();
    if deletions > 0 && deletions == length {
        report.warn(
            "deletion_would_empty_word",
            format!(
                "rule `{}` matched every segment of `{}` for deletion; the whole deletion \
                 set was refused, because a word of zero segments is unrepresentable \
                 (`lexicon.empty_form` is an Error)",
                rule.id, entry.id
            ),
        );
        let (deletes, keeps): (Vec<Edit>, Vec<Edit>) = edits
            .into_iter()
            .partition(|e| matches!(e, Edit::Delete { .. }));
        edits = keeps;
        for edit in deletes {
            blocked.push(BlockedSite {
                at: edit.at() as u32,
                reason: BlockReason::WouldEmptyWord,
            });
        }
    }

    if edits.is_empty() && blocked.is_empty() {
        return None;
    }

    let had_primary = word
        .syllables
        .iter()
        .any(|s| s.stress == Some(Stress::Primary));

    // Commit descending by snapshot index, so pending indices never shift.
    let mut sites: Vec<SiteTrace> = Vec::with_capacity(edits.len());
    edits.sort_by_key(|e| std::cmp::Reverse(e.at()));
    for edit in edits {
        match edit {
            Edit::Replace {
                at,
                with,
                resolution,
                left,
                right,
            } => {
                let before = snapshot
                    .segment_at(at)
                    .expect("site index is a snapshot index")
                    .clone();
                word.replace_at(at, with.clone());
                sites.push(SiteTrace {
                    at: at as u32,
                    before,
                    after: Some(with),
                    resolution: Some(resolution),
                    left,
                    right,
                    emptied_syllable: None,
                });
            }
            Edit::Delete { at, left, right } => {
                let before = snapshot
                    .segment_at(at)
                    .expect("site index is a snapshot index")
                    .clone();
                // Record the pattern of a syllable this deletion will empty —
                // history in the derivation, not a husk in the data.
                let emptied_syllable = word.locate(at).and_then(|(s, _)| {
                    let syllable = &word.syllables[s];
                    (syllable.segments.len() == 1).then(|| syllable.pattern.clone())
                });
                word.remove_at(at);
                sites.push(SiteTrace {
                    at: at as u32,
                    before,
                    after: None,
                    resolution: None,
                    left,
                    right,
                    emptied_syllable,
                });
            }
        }
    }
    // Sites are recorded leftmost-first whatever order they committed in.
    sites.sort_by_key(|s| s.at);

    let has_primary_now = word
        .syllables
        .iter()
        .any(|s| s.stress == Some(Stress::Primary));
    if had_primary && !has_primary_now {
        report.note(
            "stress_lost",
            format!(
                "a deletion removed `{}`'s only stressed syllable; the word now carries \
                 no primary stress, and later stress-conditioned rules will not match it",
                entry.id
            ),
        );
    }

    Some(RuleApplication {
        rule: rule.id.clone(),
        index: global_index,
        sites,
        blocked,
    })
}

/// Whether the rule's environment holds at `flat` in the snapshot.
fn environment_matches(
    environment: &Environment,
    snapshot: &Root,
    flat: usize,
    resolver: &Resolver<'_>,
) -> bool {
    let length = snapshot.len() as isize;

    for (j, item) in environment.before.iter().enumerate() {
        let index = flat as isize - 1 - j as isize;
        if !env_item_matches(item, index, -1, snapshot, resolver) {
            return false;
        }
        let _ = length; // before side bounds are handled inside env_item_matches
    }
    for (j, item) in environment.after.iter().enumerate() {
        let index = flat as isize + 1 + j as isize;
        if !env_item_matches(item, index, length, snapshot, resolver) {
            return false;
        }
    }
    true
}

/// One window slot. `edge` is the index that means "the word boundary" for this
/// side: `-1` on the left, `len` on the right.
fn env_item_matches(
    item: &EnvItem,
    index: isize,
    edge: isize,
    snapshot: &Root,
    resolver: &Resolver<'_>,
) -> bool {
    match item {
        // Only an explicit Boundary matches the edge, and it matches nothing else.
        EnvItem::Boundary => index == edge,
        EnvItem::Segment(pattern) => {
            if index < 0 || index >= snapshot.len() as isize {
                // A slot past the edge never matches a segment pattern — it is not
                // coerced to a boundary.
                return false;
            }
            let index = index as usize;
            let Some(id) = snapshot.segment_at(index) else {
                return false;
            };
            let Some(phoneme) = resolver.phoneme(id) else {
                return false;
            };
            if !phoneme.features.subsumes(pattern.features) {
                return false;
            }
            if let Some(required) = pattern.stress
                && snapshot.syllable_at(index).and_then(|s| s.stress) != Some(required)
            {
                return false;
            }
            true
        }
    }
}

/// The matched context on one side, outward from the target. `None` is the edge.
fn context_of(
    snapshot: &Root,
    flat: usize,
    slots: usize,
    left_side: bool,
) -> Vec<Option<stem_core::PhonemeId>> {
    (0..slots)
        .map(|j| {
            let index = if left_side {
                flat as isize - 1 - j as isize
            } else {
                flat as isize + 1 + j as isize
            };
            if index < 0 || index >= snapshot.len() as isize {
                None
            } else {
                snapshot.segment_at(index as usize).cloned()
            }
        })
        .collect()
}

/// The snapshot index a fixed-offset position names, given the target's index.
fn position_index(position: Position, flat: usize) -> Option<usize> {
    match position {
        Position::Before(n) => flat.checked_sub(usize::from(n) + 1),
        Position::After(n) => Some(flat + usize::from(n) + 1),
    }
}

fn resolution_id(resolution: &stem_lexicon::SymbolResolution) -> stem_core::PhonemeId {
    match resolution {
        stem_lexicon::SymbolResolution::Inventory { phoneme, .. }
        | stem_lexicon::SymbolResolution::Innovated { phoneme, .. } => phoneme.clone(),
    }
}

/// §9.5 `ambiguous_target_symbol`: more than one inventory phoneme carries the
/// output bundle, and first-in-authored-order was chosen. Saying so out loud is
/// the point — first-declared-wins with **no diagnostic** is Lexurgy issue #9,
/// and `identical_features` is deliberately only a Warning (length, tone and
/// phonation are unmodelled), so a valid language can reach this. Warned once
/// per (rule, chosen phoneme) per run; the per-site record is in the trace's
/// `ambiguous_with` regardless.
fn note_ambiguity(
    report: &mut ValidationReport,
    warned: &mut Vec<(RuleId, PhonemeId)>,
    rule: &SoundChangeRule,
    resolution: &SymbolResolution,
    resolver: &Resolver<'_>,
) {
    let SymbolResolution::Inventory {
        phoneme,
        ambiguous_with,
    } = resolution
    else {
        return;
    };
    if ambiguous_with.is_empty() {
        return;
    }
    let key = (rule.id.clone(), phoneme.clone());
    if warned.contains(&key) {
        return;
    }
    let ipa_of = |id: &PhonemeId| {
        resolver
            .phoneme(id)
            .map(|p| format!("/{}/ (`{id}`)", p.ipa))
            .unwrap_or_else(|| format!("`{id}`"))
    };
    let others = ambiguous_with
        .iter()
        .map(ipa_of)
        .collect::<Vec<_>>()
        .join(", ");
    report.warn(
        "ambiguous_target_symbol",
        format!(
            "rule `{}` produced a bundle carried by {} phonemes of this inventory; {} was \
             chosen as first in authored order, over {} — the features cannot tell them \
             apart, so every such site resolves the same way",
            rule.id,
            ambiguous_with.len() + 1,
            ipa_of(phoneme),
            others
        ),
    );
    warned.push(key);
}

/// The run-report entry for a refusal, so the CLI surfaces it without the user
/// asking for a trace.
fn note_block(
    report: &mut ValidationReport,
    rule: &SoundChangeRule,
    entry: &WordEntry,
    flat: usize,
    reason: &BlockReason,
) {
    let (code, message) = match reason {
        BlockReason::Unnameable { bundle } => (
            "unnameable_output",
            format!(
                "rule `{}` on `{}` at segment {flat}: no inventory phoneme and no \
                 reference row carries [{bundle}]; the site was left alone (add a \
                 reviewed reference row to name it)",
                rule.id, entry.id
            ),
        ),
        BlockReason::IllFormed { missing } => (
            "ill_formed_output",
            format!(
                "rule `{}` on `{}` at segment {flat}: the output would leave {} unvalued, \
                 which the validator rejects; the site was refused",
                rule.id,
                entry.id,
                missing.join(", ")
            ),
        ),
        BlockReason::SymbolHeld { ipa, by } => (
            "symbol_held",
            format!(
                "rule `{}` on `{}` at segment {flat}: minting would reuse /{ipa}/, already \
                 written by `{by}` with a different bundle; the site was refused rather \
                 than making the inventory invalid",
                rule.id, entry.id
            ),
        ),
        BlockReason::IdHeld { id } => (
            "id_held",
            format!(
                "rule `{}` on `{}` at segment {flat}: the reference id `{id}` is already \
                 declared with a different bundle; the site was refused",
                rule.id, entry.id
            ),
        ),
        BlockReason::DonorHasNoNode { node, donor } => (
            "donor_has_no_node",
            format!(
                "rule `{}` on `{}` at segment {flat}: the donor `{donor}` values no \
                 articulator of the {node} node, so there is nothing to copy",
                rule.id, entry.id
            ),
        ),
        // Reported once per word at set level, not per site.
        BlockReason::WouldEmptyWord => return,
    };
    let severity = match reason {
        BlockReason::DonorHasNoNode { .. } => stem_core::Severity::Note,
        _ => stem_core::Severity::Warning,
    };
    report.push(stem_core::Issue::new(severity, code, message));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::{EnvItem, Environment, Position, SegmentPattern};
    use stem_core::RuleId;
    use stem_lexicon::{ConceptKey, WordEntry, WordSource};
    use stem_phonology::prosody::WordEdge;
    use stem_phonology::{FeatureBundle, FeatureNode, Phoneme, SegmentKind, Syllable};

    fn bundle(tokens: &[&str]) -> FeatureBundle {
        FeatureBundle::try_from(tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
            .expect("valid feature list")
    }

    /// /t k j a e i/ with the reference fixture's own bundles. /i/ and /j/ differ
    /// in exactly `syllabic`, which several tests exploit.
    fn inventory() -> PhonemeInventory {
        let seg = |id: &str, ipa: &str, kind, tokens: &[&str]| {
            Phoneme::new(id, ipa, kind).with_features(bundle(tokens))
        };
        PhonemeInventory::from_phonemes([
            seg(
                "ph_t",
                "t",
                SegmentKind::Consonant,
                &[
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
                    "+coronal",
                    "-dorsal",
                ],
            ),
            seg(
                "ph_k",
                "k",
                SegmentKind::Consonant,
                &[
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
                    "+dorsal",
                    "+high",
                    "-low",
                    "+back",
                    "-round",
                ],
            ),
            seg(
                "ph_j",
                "j",
                SegmentKind::Consonant,
                &[
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
                    "-back",
                    "-round",
                ],
            ),
            seg(
                "ph_a",
                "a",
                SegmentKind::Vowel,
                &[
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
                    "+back",
                    "-round",
                ],
            ),
            seg(
                "ph_e",
                "e",
                SegmentKind::Vowel,
                &[
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
                    "-low",
                    "-back",
                    "-round",
                ],
            ),
            seg(
                "ph_i",
                "i",
                SegmentKind::Vowel,
                &[
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
                    "-back",
                    "-round",
                ],
            ),
        ])
    }

    fn word(id: &str, syllables: &[&[&str]]) -> WordEntry {
        WordEntry {
            id: stem_core::WordId::new(id),
            concept: Some(ConceptKey::new("STAR")),
            phonemic_form: Root {
                syllables: syllables
                    .iter()
                    .map(|segs| Syllable {
                        pattern: "V".to_owned(),
                        segments: segs.iter().map(|s| stem_core::PhonemeId::new(*s)).collect(),
                        stress: None,
                    })
                    .collect(),
            },
            glosses: vec!["star".to_owned()],
            part_of_speech: stem_lexicon::PartOfSpeech::Noun,
            cognate_set: stem_core::CognateSetId::new("cog_x_0001"),
            source: WordSource::Authored,
            trace: None,
            morphemes: Vec::new(),
        }
    }

    fn pattern(tokens: &[&str]) -> SegmentPattern {
        SegmentPattern {
            features: bundle(tokens),
            stress: None,
        }
    }

    fn rule(
        id: &str,
        target: SegmentPattern,
        environment: Environment,
        change: Change,
    ) -> SoundChangeRule {
        SoundChangeRule {
            id: RuleId::new(id),
            name: id.to_owned(),
            description: String::new(),
            chronology_years: 0,
            target,
            environment,
            change,
        }
    }

    fn run(rules: &[SoundChangeRule], entry: WordEntry) -> Evolution {
        let inventory = inventory();
        apply_rules(
            rules,
            0,
            &inventory,
            &Prosody::new(),
            &Lexicon::from_entries([entry]),
        )
        .expect("applies")
    }

    fn written(evolution: &Evolution) -> String {
        evolution
            .lexicon
            .iter()
            .next()
            .unwrap()
            .phonemic_form
            .written(&evolution.inventory)
            .unwrap()
    }

    /// `iii` under `[+syllabic] to [-syllabic] / [+syllabic] _`: positions 1 and 2
    /// both see an `i` to their left **in the snapshot**, so both become /j/.
    /// Iterative-with-feeding would change only position 1.
    #[test]
    fn a_rule_does_not_feed_itself_within_one_pass() {
        let glide = rule(
            "r_glide",
            pattern(&["+syllabic"]),
            Environment {
                before: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
                after: vec![],
            },
            Change::Set(bundle(&["-syllabic"])),
        );
        let evolution = run(&[glide], word("w_1", &[&["ph_i"], &["ph_i"], &["ph_i"]]));
        assert_eq!(written(&evolution), "ijj");
    }

    /// `aaaa` under `V to nothing / V _` deletes every matched site — 1, 2 and 3 —
    /// not alternating ones.
    #[test]
    fn simultaneous_deletion_removes_every_matched_site_not_alternating_ones() {
        let del = rule(
            "r_del",
            pattern(&["+syllabic"]),
            Environment {
                before: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
                after: vec![],
            },
            Change::Delete,
        );
        let evolution = run(
            &[del],
            word("w_1", &[&["ph_a"], &["ph_a"], &["ph_a"], &["ph_a"]]),
        );
        assert_eq!(written(&evolution), "a");
    }

    /// The height-harmony case that separates snapshot-donor from live-donor:
    /// `a e a` under `V copies place from after(0) / _ V`. With a snapshot donor,
    /// position 0 copies /e/'s place and position 1 copies /a/'s: `eaa`. A
    /// right-to-left live donor would give `aaa`.
    #[test]
    fn a_copy_reads_its_donor_from_the_pre_rule_snapshot() {
        let harmony = rule(
            "r_harm",
            pattern(&["+syllabic"]),
            Environment {
                before: vec![],
                after: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
            },
            Change::Copy {
                from: Position::After(0),
                node: FeatureNode::Place,
            },
        );
        let evolution = run(&[harmony], word("w_1", &[&["ph_a"], &["ph_e"], &["ph_a"]]));
        assert_eq!(written(&evolution), "eaa");
    }

    /// A rule that would delete every segment of a word is refused **as a set**,
    /// and the word survives unchanged with the refusals traced.
    #[test]
    fn a_rule_that_would_delete_every_segment_of_a_word_is_refused_as_a_set() {
        let all = rule(
            "r_all",
            pattern(&["+syllabic"]),
            Environment::default(),
            Change::Delete,
        );
        let evolution = run(&[all], word("w_1", &[&["ph_a"], &["ph_a"]]));
        assert_eq!(written(&evolution), "aa", "the word must survive");

        let entry = evolution.lexicon.iter().next().unwrap();
        let step = &entry.trace.as_ref().expect("traced").steps[0];
        assert!(step.sites.is_empty());
        assert_eq!(step.blocked.len(), 2);
        assert!(
            step.blocked
                .iter()
                .all(|b| matches!(b.reason, stem_lexicon::BlockReason::WouldEmptyWord)),
            "{step:?}"
        );
        assert!(
            evolution
                .report
                .warnings()
                .any(|i| i.code == "deletion_would_empty_word"),
            "{}",
            evolution.report
        );
    }

    /// A segment minted by an earlier rule must be matchable by a later one, or
    /// feeding order — the whole point of ordered sound change — is broken.
    #[test]
    fn a_segment_minted_by_an_earlier_rule_is_matched_by_a_later_one() {
        let intervocalic = |id: &str, target: SegmentPattern, change: Change| {
            rule(
                id,
                target,
                Environment {
                    before: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
                    after: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
                },
                change,
            )
        };
        let voice = intervocalic(
            "r_voice",
            pattern(&["-sonorant", "-continuant", "-voice", "+dorsal"]),
            Change::Set(bundle(&["+voice"])),
        );
        let lenite = intervocalic(
            "r_lenite",
            pattern(&["-sonorant", "-continuant", "+voice", "+dorsal"]),
            Change::Set(bundle(&["+continuant"])),
        );
        // a k a — no voiced stop exists; only rule 1 can create rule 2's target.
        let evolution = run(
            &[voice, lenite],
            word("w_1", &[&["ph_a"], &["ph_k"], &["ph_a"]]),
        );
        let entry = evolution.lexicon.iter().next().unwrap();
        assert_eq!(
            entry.phonemic_form.ipa(&evolution.inventory).unwrap(),
            "a\u{0263}a"
        );
        assert_eq!(
            entry.trace.as_ref().unwrap().steps.len(),
            2,
            "both rules applied, the second to a segment the first minted"
        );
    }

    /// `Some(Unstressed)` matches only a syllable explicitly marked unstressed —
    /// a language with no prosody never matches.
    #[test]
    fn an_unmarked_syllable_does_not_match_a_rule_conditioned_on_unstressed() {
        let apocope = rule(
            "r_apoc",
            SegmentPattern {
                features: bundle(&["+syllabic"]),
                stress: Some(stem_phonology::Stress::Unstressed),
            },
            Environment {
                before: vec![],
                after: vec![EnvItem::Boundary],
            },
            Change::Delete,
        );
        let evolution = run(&[apocope], word("w_1", &[&["ph_a"], &["ph_a"]]));
        assert_eq!(written(&evolution), "aa");
    }

    /// A monosyllable's only syllable takes primary stress under a fixed policy,
    /// so its vowel does not match `unstressed` and survives — the difference
    /// between "final unstressed vowel loss" and "delete the last vowel".
    #[test]
    fn a_monosyllable_keeps_its_final_vowel_because_that_vowel_is_stressed() {
        let apocope = rule(
            "r_apoc",
            SegmentPattern {
                features: bundle(&["+syllabic"]),
                stress: Some(stem_phonology::Stress::Unstressed),
            },
            Environment {
                before: vec![],
                after: vec![EnvItem::Boundary],
            },
            Change::Delete,
        );
        let inventory = inventory();
        let prosody = Prosody::fixed(WordEdge::Left, 0);

        let evolution = apply_rules(
            &[apocope],
            0,
            &inventory,
            &prosody,
            &Lexicon::from_entries([
                word("w_1", &[&["ph_a"]]),
                word("w_2", &[&["ph_a"], &["ph_a"]]),
            ]),
        )
        .expect("applies");

        let forms: Vec<String> = evolution
            .lexicon
            .iter()
            .map(|e| e.phonemic_form.written(&evolution.inventory).unwrap())
            .collect();
        assert_eq!(
            forms,
            ["a", "a"],
            "the monosyllable kept its stressed vowel; the disyllable lost its \
             unstressed final"
        );
    }

    /// A word no rule touched carries `Some(steps: [])` — "carried through
    /// unchanged" — not `None`, which means "never passed through a rule".
    #[test]
    fn a_word_no_rule_touched_carries_an_empty_derivation_not_a_missing_one() {
        let voice = rule(
            "r_voice",
            pattern(&["-sonorant", "-continuant", "-voice", "+dorsal"]),
            Environment {
                before: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
                after: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
            },
            Change::Set(bundle(&["+voice"])),
        );
        // /t a/ — the rule targets dorsals, so nothing matches.
        let evolution = run(&[voice], word("w_1", &[&["ph_t", "ph_a"]]));
        let entry = evolution.lexicon.iter().next().unwrap();
        let derivation = entry.trace.as_ref().expect("Some, not None");
        assert!(derivation.steps.is_empty(), "with no steps");
        assert!(
            evolution
                .report
                .issues
                .iter()
                .any(|i| i.code == "rule_never_applied"),
            "{}",
            evolution.report
        );
    }

    /// Two rules producing the same segment mint it once — a converging split is
    /// one phoneme, the linguistically correct merger.
    #[test]
    fn two_rules_producing_the_same_segment_mint_it_once() {
        let voice_after = |id: &str, env_feature: &str| {
            rule(
                id,
                pattern(&["-sonorant", "-continuant", "-voice", "+dorsal"]),
                Environment {
                    before: vec![EnvItem::Segment(pattern(&[env_feature]))],
                    after: vec![],
                },
                Change::Set(bundle(&["+voice"])),
            )
        };
        // a k | i k — rule 1 fires after /a/ (+low), rule 2 after /i/ (+high);
        // both outputs are the /g/ bundle.
        let evolution = run(
            &[voice_after("r_1", "+low"), voice_after("r_2", "+high")],
            word("w_1", &[&["ph_a", "ph_k"], &["ph_i", "ph_k"]]),
        );
        let minted: Vec<&str> = evolution
            .inventory
            .iter()
            .filter(|p| p.id.as_str() == "ph_g")
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(minted, ["ph_g"], "one phoneme, not one per rule");
    }

    #[test]
    fn the_input_inventory_is_never_mutated_by_a_rule_run() {
        let before = inventory();
        let voice = rule(
            "r_voice",
            pattern(&["-sonorant", "-continuant", "-voice", "+dorsal"]),
            Environment {
                before: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
                after: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
            },
            Change::Set(bundle(&["+voice"])),
        );
        let evolution = apply_rules(
            &[voice],
            0,
            &before,
            &Prosody::new(),
            &Lexicon::from_entries([word("w_1", &[&["ph_a"], &["ph_k"], &["ph_a"]])]),
        )
        .expect("applies");
        assert_eq!(before, inventory(), "the input is untouched");
        assert_eq!(
            evolution.inventory.len(),
            before.len() + 1,
            "the mint lives only in the returned inventory"
        );
    }

    #[test]
    fn every_segment_of_every_evolved_word_resolves_in_the_returned_inventory() {
        let voice = rule(
            "r_voice",
            pattern(&["-sonorant", "-continuant", "-voice", "+dorsal"]),
            Environment {
                before: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
                after: vec![EnvItem::Segment(pattern(&["+syllabic"]))],
            },
            Change::Set(bundle(&["+voice"])),
        );
        let evolution = run(&[voice], word("w_1", &[&["ph_a"], &["ph_k"], &["ph_a"]]));
        for entry in evolution.lexicon.iter() {
            for id in entry.phonemic_form.segments() {
                assert!(evolution.inventory.get(id).is_some(), "{id} unresolvable");
            }
        }
    }

    /// An engine refusal and a validator rejection must be the same set of
    /// languages: `apply_rules` gates on the unfiltered report.
    #[test]
    fn a_language_the_engine_refuses_is_exactly_a_language_validate_calls_broken() {
        let mut broken = inventory();
        broken.push(Phoneme::new("ph_x", "x", SegmentKind::Consonant)); // featureless
        assert!(!broken.validate().is_ok());

        let voice = rule(
            "r_voice",
            pattern(&["+syllabic"]),
            Environment::default(),
            Change::Set(bundle(&["+voice"])),
        );
        assert!(
            apply_rules(
                &[voice],
                0,
                &broken,
                &Prosody::new(),
                &Lexicon::from_entries([word("w_1", &[&["ph_a"]])]),
            )
            .is_err(),
            "validate says Error, so the engine must refuse"
        );
    }

    /// §9.5 `ambiguous_target_symbol`. /a/ and /aː/ share a bundle — legal,
    /// `identical_features` is deliberately only a Warning — so a rule resolving
    /// to that bundle picks first-in-authored-order and must say so in the run
    /// report: first-declared-wins with no diagnostic is Lexurgy issue #9. Once
    /// per (rule, chosen phoneme), not once per site; the per-site record lives
    /// in the trace.
    #[test]
    fn a_bundle_two_phonemes_share_warns_ambiguous_target_symbol_once() {
        let mut ambiguous = inventory();
        let mut a_long = ambiguous
            .get(&stem_core::PhonemeId::new("ph_a"))
            .expect("ph_a is declared")
            .clone();
        a_long.id = stem_core::PhonemeId::new("ph_a_long");
        a_long.ipa = "aː".to_owned();
        ambiguous.push(a_long);
        assert!(
            ambiguous.validate().is_ok(),
            "identical bundles must stay legal or this test proves nothing"
        );

        // /e/ is [-high, -low, -back]; overlaying [+low, +back] lands exactly on
        // the bundle /a/ and /aː/ share. Two sites, to make the dedup observable.
        let lower = rule(
            "r_lower",
            pattern(&["+syllabic", "-high", "-low", "-back"]),
            Environment::default(),
            Change::Set(bundle(&["+low", "+back"])),
        );
        let evolution = apply_rules(
            &[lower],
            0,
            &ambiguous,
            &Prosody::new(),
            &Lexicon::from_entries([word("w_amb", &[&["ph_e"], &["ph_e"]])]),
        )
        .expect("applies");

        let entry = evolution.lexicon.iter().next().expect("one word");
        assert!(
            entry
                .phonemic_form
                .segments()
                .all(|id| id == &stem_core::PhonemeId::new("ph_a")),
            "both sites resolve to the first-declared /a/"
        );

        let warned = evolution
            .report
            .issues
            .iter()
            .filter(|i| i.code == "ambiguous_target_symbol")
            .count();
        assert_eq!(
            warned, 1,
            "one warning for two identical sites, not zero and not two: {}",
            evolution.report
        );

        let step = &entry.trace.as_ref().expect("derived").steps[0];
        assert!(
            step.sites.iter().all(|site| matches!(
                &site.resolution,
                Some(stem_lexicon::SymbolResolution::Inventory { ambiguous_with, .. })
                    if ambiguous_with == &[stem_core::PhonemeId::new("ph_a_long")]
            )),
            "every site's trace records what /a/ was ambiguous with"
        );
    }
}
