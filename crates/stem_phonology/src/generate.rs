//! Seeded, weighted root generation (ROADMAP M1, `DESIGN.md` §15 Ticket 4).
//!
//! # The draw-order contract
//!
//! This is normative. If it drifts, every previously generated lexicon is silently
//! invalidated, so it is written down here, pinned by
//! `generating_fewer_roots_is_a_prefix_of_generating_more`, and any change to it
//! requires a `PROGRESS.md` entry.
//!
//! Every random decision goes through **one** primitive: a weighted-index sample
//! over exact `u32` weights. There is no range draw, no boolean draw, no float
//! comparison, and no rejection loop. Every distribution is built once, before the
//! first draw, so the prefix-sum arrays are identical for every root.
//!
//! For each root, in order, from a single [`StemmaRng`] threaded by `&mut`:
//!
//! 1. **one** draw — the syllable count, over `syllables_per_root` in authored
//!    order;
//! 2. then, for each syllable left to right:
//!    - (a) **one** draw — the template, over `templates` in authored order;
//!    - (b) then, for each slot of that template left to right, **one** draw — the
//!      segment, over the consonant or vowel candidates in **authored inventory
//!      order**.
//!
//! Steps 2a and 2b interleave per syllable; the template is redrawn for every
//! syllable, not once per root. The generator is never re-seeded per root, so
//! `generate(n)` is a strict prefix of `generate(n + k)`.
//!
//! Authored inventory order is not a stylistic preference. Reversing a weight
//! array and remapping the indices back changes almost every draw, because the
//! prefix-sum boundaries land at different points in `[0, total)`. Any refactor
//! that reorders the inventory, iterates a map, or changes a sort's tie-breaking
//! rewrites every generated language.
//!
//! The weight *type* is part of the contract too: a `u32` weighted index and a
//! `u64` one produce different sequences from the same stream. `u32` is frozen.

use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use stem_core::rng::StemmaRng;
use stem_core::{PhonemeId, Result, StemmaError, Validate, ValidationReport};

use crate::PhonemeInventory;
use crate::phoneme::SegmentKind;
use crate::phonotactics::{Phonotactics, Slot, check_against_inventory};

/// Inventory issues that actually prevent generation.
///
/// M1's generator reads `kind` and `frequency_weight` and nothing else, so a
/// feature fault — however real — cannot change a single draw. Blocking on one
/// would mean a working pre-M1 file stops generating the moment its author starts
/// adding features to it, one phoneme at a time.
fn generation_blocking(report: ValidationReport) -> ValidationReport {
    /// Codes about features rather than about generation. Kept as an explicit
    /// list so adding a feature check does not silently start blocking generation.
    const FEATURE_ONLY: &[&str] = &[
        "features_unspecified",
        "missing_required_feature",
        "identical_features",
        "nucleus_not_syllabic",
        "round_without_labial",
    ];

    ValidationReport {
        issues: report
            .issues
            .into_iter()
            .filter(|issue| !FEATURE_ONLY.contains(&issue.code.as_str()))
            .collect(),
    }
}

/// One syllable of a generated root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Syllable {
    /// The template this syllable was built to, e.g. `"CVC"`.
    pub pattern: String,
    /// Its segments, in order.
    pub segments: Vec<PhonemeId>,
}

/// One generated root.
///
/// Segments are [`PhonemeId`]s rather than a string because M3's rules operate on
/// segments and `CLAUDE.md`'s traceability constraint requires every form to be
/// walkable back to what it is made of. The string is a *rendering*.
///
/// The syllabification is carried rather than recomputed: it is what the generator
/// actually did, and re-deriving it from the output would be a check that cannot
/// fail. It is also the seed of M3's prosodic domain — a root already knows its
/// own syllable structure, which is where stress will hang.
///
/// `Ord` and `Hash` are derived so M2 can key a set of roots for deduplication
/// without a breaking change. They order by syllable structure, not by rendered
/// form — rendering needs an inventory, which a derive cannot have.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Root {
    /// The syllables, in order.
    pub syllables: Vec<Syllable>,
}

impl Root {
    /// Every segment, flattened.
    pub fn segments(&self) -> impl Iterator<Item = &PhonemeId> {
        self.syllables.iter().flat_map(|s| s.segments.iter())
    }

    /// How many segments the root has.
    pub fn len(&self) -> usize {
        self.syllables.iter().map(|s| s.segments.len()).sum()
    }

    /// Whether the root has no segments. Never true for a generated root.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The romanised form, e.g. `"takala"`.
    ///
    /// Returns `Result` because this is public API over an arbitrary inventory: the
    /// ids in a root produced by [`RootGenerator`] are guaranteed present in *that*
    /// generator's inventory, but nothing stops a caller passing a different one.
    /// Uses [`PhonemeInventory::require`], never `is_ok_and` or `unwrap_or_default`
    /// — silently rendering a missing segment as nothing would corrupt the form.
    pub fn written(&self, inventory: &PhonemeInventory) -> Result<String> {
        self.render(inventory, |p| p.written())
    }

    /// The IPA form.
    pub fn ipa(&self, inventory: &PhonemeInventory) -> Result<String> {
        self.render(inventory, |p| p.ipa.as_str())
    }

    fn render(
        &self,
        inventory: &PhonemeInventory,
        part: impl Fn(&crate::Phoneme) -> &str,
    ) -> Result<String> {
        let mut out = String::new();
        for id in self.segments() {
            out.push_str(part(inventory.require(id)?));
        }
        Ok(out)
    }

    /// The slot classes of each syllable, for checking against
    /// [`Phonotactics::admits_syllable`].
    pub fn syllable_kinds(&self, inventory: &PhonemeInventory) -> Result<Vec<Vec<SegmentKind>>> {
        self.syllables
            .iter()
            .map(|syllable| {
                syllable
                    .segments
                    .iter()
                    .map(|id| inventory.require(id).map(|p| p.kind))
                    .collect()
            })
            .collect()
    }
}

/// A generator over one inventory and one phonotactic system.
///
/// Construction validates; generation cannot fail. Making an unsatisfiable
/// configuration *unconstructible* beats bounded retries: there is no rejection
/// sampling anywhere in M1, so generation is total and its draw count is a pure
/// function of the draws already made — which is what makes the prefix property
/// hold.
///
/// Takes `&PhonemeInventory` and `&Phonotactics` rather than a genome, so
/// `stem_phonology` stays below `stem_genome` and the crate DAG is unchanged.
#[derive(Debug)]
pub struct RootGenerator<'a> {
    inventory: &'a PhonemeInventory,
    /// Parsed once at construction, in authored order.
    templates: Vec<(String, Vec<Slot>)>,
    template_dist: WeightedIndex<u32>,
    counts: Vec<u8>,
    count_dist: WeightedIndex<u32>,
    /// The consonant candidates, or `None` when no template has a `C` slot.
    consonants: Option<Candidates>,
    /// The vowel candidates, or `None` when no template has a `V` slot.
    vowels: Option<Candidates>,
}

/// The draw table for one slot class: ids in **authored inventory order** and the
/// distribution over their weights. Never a map — authored order is part of the
/// determinism contract.
#[derive(Debug)]
struct Candidates {
    ids: Vec<PhonemeId>,
    dist: WeightedIndex<u32>,
}

impl Candidates {
    fn draw(&self, rng: &mut StemmaRng) -> PhonemeId {
        self.ids[self.dist.sample(rng)].clone()
    }
}

impl<'a> RootGenerator<'a> {
    /// Builds a generator, or explains why the language cannot generate.
    ///
    /// Fails with [`StemmaError::Invalid`] carrying the full report, so a bad
    /// configuration is *explained* rather than merely refused — the same
    /// convention as every other load path in the workspace.
    pub fn new(inventory: &'a PhonemeInventory, phonotactics: &'a Phonotactics) -> Result<Self> {
        let mut report = ValidationReport::new();
        report.absorb("phonotactics", phonotactics.validate());
        report.absorb(
            "phonotactics",
            check_against_inventory(phonotactics, inventory),
        );
        // Only the inventory checks that bear on *generation*. The feature checks
        // are deliberately excluded: M1's generator reads `kind` and
        // `frequency_weight` and nothing else, so blocking generation on a feature
        // it never consults would refuse a working pre-M1 file for a fault that
        // cannot affect its output. `features_unspecified` is a Warning for the
        // same reason (`inventory.rs`), and this keeps the two consistent.
        report.absorb("phonology", generation_blocking(inventory.validate()));

        // `validate` reports these as Notes — a language may legitimately not have
        // declared shapes yet. Asking it to generate is what makes them fatal.
        if phonotactics.templates.is_empty() {
            report.error(
                "phonotactics.no_templates",
                "cannot generate roots: no syllable templates are declared",
            );
        }
        if phonotactics.syllables_per_root.is_empty() {
            report.error(
                "phonotactics.no_syllable_counts",
                "cannot generate roots: no root lengths are declared",
            );
        }

        if !report.is_ok() {
            return Err(StemmaError::Invalid(
                "this language cannot generate roots".to_owned(),
                report,
            ));
        }

        let templates: Vec<(String, Vec<Slot>)> = phonotactics
            .templates
            .iter()
            .map(|t| {
                // Unreachable: `bad_template` is an Error and we returned above.
                let slots = t.slots().expect("templates validated above");
                (t.pattern.clone(), slots)
            })
            .collect();

        let dist = |weights: Vec<u32>, what: &str| -> Result<WeightedIndex<u32>> {
            WeightedIndex::new(weights).map_err(|e| {
                let mut report = ValidationReport::new();
                report.error("phonology.weight_distribution", format!("{what}: {e}"));
                StemmaError::Invalid("this language cannot generate roots".to_owned(), report)
            })
        };

        // A slot class is only prepared if some template actually uses it.
        //
        // Building both unconditionally would refuse a vowel-only language — which
        // `validate` deliberately passes with only a `no_consonants` *warning*,
        // because CLAUDE.md's rule is that unusual designs are flagged, not
        // rejected. An all-vowel inventory is typologically unattested but it is
        // not broken, and `V`/`VV` templates over it are perfectly generatable.
        // A template that needs a class the inventory cannot fill has already been
        // caught above as `phonotactics.slot_unsatisfiable`.
        let needed = |slot: Slot| templates.iter().any(|(_, slots)| slots.contains(&slot));

        let candidates = |slot: Slot, what: &str| -> Result<Option<Candidates>> {
            if !needed(slot) {
                return Ok(None);
            }
            let (ids, weights) = inventory.candidates(slot.kind());
            Ok(Some(Candidates {
                dist: dist(weights, what)?,
                ids,
            }))
        };

        Ok(Self {
            inventory,
            template_dist: dist(
                phonotactics.templates.iter().map(|t| t.weight).collect(),
                "syllable template weights",
            )?,
            counts: phonotactics
                .syllables_per_root
                .iter()
                .map(|c| c.count)
                .collect(),
            count_dist: dist(
                phonotactics
                    .syllables_per_root
                    .iter()
                    .map(|c| c.weight)
                    .collect(),
                "root length weights",
            )?,
            consonants: candidates(Slot::Consonant, "consonant weights")?,
            vowels: candidates(Slot::Vowel, "vowel weights")?,
            templates,
        })
    }

    /// The draw table for a slot, which construction guarantees exists.
    fn candidates_for(&self, slot: Slot) -> &Candidates {
        let table = match slot {
            Slot::Consonant => self.consonants.as_ref(),
            Slot::Vowel => self.vowels.as_ref(),
        };
        table.expect(
            "unreachable: `new` prepares a slot class iff some template uses it, and \
             rejects any template whose class the inventory cannot fill",
        )
    }

    /// The inventory this generator draws from.
    pub fn inventory(&self) -> &PhonemeInventory {
        self.inventory
    }

    /// One root. Infallible by construction; follows the module's draw order
    /// exactly.
    pub fn next_root(&self, rng: &mut StemmaRng) -> Root {
        // 1. one draw: the syllable count.
        let syllable_count = self.counts[self.count_dist.sample(rng)];

        let mut syllables = Vec::with_capacity(syllable_count as usize);
        for _ in 0..syllable_count {
            // 2a. one draw: the template, redrawn per syllable.
            let (pattern, slots) = &self.templates[self.template_dist.sample(rng)];

            // 2b. one draw per slot, left to right.
            let segments = slots
                .iter()
                .map(|&slot| self.candidates_for(slot).draw(rng))
                .collect();

            syllables.push(Syllable {
                pattern: pattern.clone(),
                segments,
            });
        }

        Root { syllables }
    }

    /// `count` roots, in order.
    ///
    /// Duplicates are possible and are returned as generated. M1 has no lexicon to
    /// key, and silently dropping collisions would make `--count 100` return fewer
    /// than 100 — and would make the draw count depend on the set of roots already
    /// produced, a far harder determinism story for no M1 benefit. Homophony is
    /// also real. Deduplication belongs to M2, where `WordEntry` gives it a home.
    pub fn generate(&self, rng: &mut StemmaRng, count: usize) -> Vec<Root> {
        (0..count).map(|_| self.next_root(rng)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phonotactics::{WeightedSyllableCount, WeightedTemplate};
    use crate::{Phoneme, SegmentKind};
    use stem_core::{RngDomain, rng_for};

    fn inventory() -> PhonemeInventory {
        PhonemeInventory::from_phonemes([
            Phoneme::new("ph_t", "t", SegmentKind::Consonant).with_weight(50),
            Phoneme::new("ph_k", "k", SegmentKind::Consonant).with_weight(45),
            Phoneme::new("ph_m", "m", SegmentKind::Consonant).with_weight(35),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel).with_weight(60),
            Phoneme::new("ph_i", "i", SegmentKind::Vowel).with_weight(40),
        ])
    }

    fn phonotactics() -> Phonotactics {
        Phonotactics {
            templates: vec![
                WeightedTemplate::new("CV").with_weight(45),
                WeightedTemplate::new("CVC").with_weight(35),
            ],
            syllables_per_root: vec![
                WeightedSyllableCount::new(1).with_weight(25),
                WeightedSyllableCount::new(2).with_weight(55),
            ],
        }
    }

    fn generate(seed: u64, count: usize) -> (PhonemeInventory, Vec<Root>) {
        let inventory = inventory();
        let phonotactics = phonotactics();
        let roots = {
            let generator = RootGenerator::new(&inventory, &phonotactics).expect("constructs");
            let mut rng = rng_for(seed, RngDomain::Roots);
            generator.generate(&mut rng, count)
        };
        (inventory, roots)
    }

    #[test]
    fn generating_a_hundred_roots_produces_a_hundred_roots() {
        let (_, roots) = generate(42, 100);
        assert_eq!(roots.len(), 100);
    }

    #[test]
    fn every_generated_segment_comes_from_the_inventory() {
        let (inventory, roots) = generate(42, 200);
        for root in &roots {
            for id in root.segments() {
                assert!(inventory.get(id).is_some(), "{id} is not in the inventory");
            }
        }
    }

    /// Crosses the generator/inventory boundary via `admits_syllable`, which the
    /// generator itself never calls — so this is not a tautology.
    #[test]
    fn every_generated_syllable_matches_an_authored_template() {
        let (inventory, roots) = generate(42, 200);
        let phonotactics = phonotactics();
        for root in &roots {
            for kinds in root.syllable_kinds(&inventory).expect("ids resolve") {
                assert!(
                    phonotactics.admits_syllable(&kinds),
                    "syllable {kinds:?} matches no declared template"
                );
            }
        }
    }

    #[test]
    fn the_same_seed_produces_byte_identical_output() {
        let (inventory, first) = generate(42, 100);
        let (_, second) = generate(42, 100);
        assert_eq!(first, second);

        let render = |roots: &[Root]| -> Vec<String> {
            roots
                .iter()
                .map(|r| r.written(&inventory).unwrap())
                .collect()
        };
        assert_eq!(render(&first), render(&second));
    }

    #[test]
    fn a_different_seed_produces_different_output() {
        let (_, a) = generate(1, 100);
        let (_, b) = generate(2, 100);
        assert_ne!(a, b);
    }

    /// Pins the draw-order contract: the generator is never re-seeded per root, so
    /// asking for fewer must be a prefix of asking for more.
    #[test]
    fn generating_fewer_roots_is_a_prefix_of_generating_more() {
        let (_, few) = generate(7, 20);
        let (_, many) = generate(7, 100);
        assert_eq!(few.as_slice(), &many[..20]);
    }

    /// Property test (`DESIGN.md` §16.3): follows from `NoNucleus` being an Error
    /// on templates, asserted end to end over many streams rather than one.
    #[test]
    fn every_root_has_at_least_one_nucleus() {
        let inventory = inventory();
        let phonotactics = phonotactics();
        let generator = RootGenerator::new(&inventory, &phonotactics).unwrap();
        for seed in 0..1_000u64 {
            let mut rng = rng_for(seed, RngDomain::Roots);
            for root in generator.generate(&mut rng, 5) {
                let nuclei = root
                    .segments()
                    .filter(|id| inventory.get(id).is_some_and(|p| p.is_nucleus()))
                    .count();
                assert!(
                    nuclei >= 1,
                    "seed {seed}: {:?} has no nucleus",
                    root.written(&inventory)
                );
            }
        }
    }

    /// The consensus defect from M1's review: a vowel-only inventory is
    /// typologically unattested, so `validate` *warns* — and CLAUDE.md says warn,
    /// do not reject. Generation must therefore work.
    #[test]
    fn a_vowel_only_language_can_still_generate() {
        let vowels = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_a", "a", SegmentKind::Vowel).with_weight(60),
            Phoneme::new("ph_i", "i", SegmentKind::Vowel).with_weight(40),
        ]);
        let phonotactics = Phonotactics {
            templates: vec![
                WeightedTemplate::new("V").with_weight(60),
                WeightedTemplate::new("VV").with_weight(40),
            ],
            syllables_per_root: vec![WeightedSyllableCount::new(2)],
        };

        // The inventory is only *warned* about, so it must not block construction.
        assert!(vowels.validate().is_ok(), "{}", vowels.validate());

        let generator = RootGenerator::new(&vowels, &phonotactics)
            .expect("a warned-about language must still generate");
        let mut rng = rng_for(0, RngDomain::Roots);
        let roots = generator.generate(&mut rng, 20);
        assert_eq!(roots.len(), 20);
        for root in &roots {
            assert!(!root.written(&vowels).unwrap().is_empty());
        }
    }

    /// A feature fault cannot change a single draw, so it must not stop generation
    /// — otherwise adding features to a working file one phoneme at a time breaks
    /// it halfway through.
    #[test]
    fn a_feature_fault_does_not_block_generation() {
        let mut partial = inventory();
        let half_featured = Phoneme::new("ph_extra", "e", SegmentKind::Vowel).with_features(
            crate::FeatureBundle::EMPTY.with(crate::Feature::Syllabic, crate::Sign::Plus),
        );
        partial.push(half_featured);

        // The inventory really is invalid — on features.
        let report = partial.validate();
        assert!(
            report
                .errors()
                .any(|i| i.code == "missing_required_feature"),
            "{report}"
        );

        // …and generation proceeds anyway, because it reads `kind` and weights only.
        let phonotactics = phonotactics();
        RootGenerator::new(&partial, &phonotactics)
            .expect("a feature fault must not block generation");
    }

    /// But a fault that *does* bear on generation still blocks it.
    #[test]
    fn a_zero_weight_still_blocks_generation() {
        let mut broken = inventory();
        broken.push(Phoneme::new("ph_zero", "z", SegmentKind::Consonant).with_weight(0));
        let phonotactics = phonotactics();
        let error = RootGenerator::new(&broken, &phonotactics).expect_err("must refuse");
        assert!(error.to_string().contains("bad_weight"), "{error}");
    }

    #[test]
    fn a_language_with_no_phonotactics_cannot_construct_a_generator() {
        let inventory = inventory();
        let empty = Phonotactics::new();
        let error = RootGenerator::new(&inventory, &empty).expect_err("must refuse");
        let text = error.to_string();
        assert!(text.contains("no_templates"), "{text}");
    }

    #[test]
    fn a_template_needing_a_consonant_the_inventory_lacks_is_rejected_at_construction() {
        let vowels_only =
            PhonemeInventory::from_phonemes([Phoneme::new("ph_a", "a", SegmentKind::Vowel)]);
        let error = RootGenerator::new(&vowels_only, &phonotactics()).expect_err("must refuse");
        assert!(error.to_string().contains("slot_unsatisfiable"), "{error}");
    }

    /// The integer-weight guarantee, stated as a test. With floats this fails.
    #[test]
    fn weights_scaled_by_a_constant_draw_identically() {
        let draw = |scale: u32| -> Vec<usize> {
            let weights: Vec<u32> = [30u32, 25, 20, 12, 8, 5]
                .iter()
                .map(|w| w * scale)
                .collect();
            let dist = WeightedIndex::new(weights).unwrap();
            let mut rng = rng_for(0, RngDomain::Roots);
            (0..500).map(|_| dist.sample(&mut rng)).collect()
        };
        assert_eq!(draw(1), draw(100));
    }

    /// Asserts the dependency **explicitly**, so the constraint is on record rather
    /// than assumed. Reordering the inventory is a breaking change.
    #[test]
    fn reordering_the_inventory_changes_the_output() {
        let phonotactics = phonotactics();

        let forward = inventory();
        let mut reversed_phonemes: Vec<_> = forward.iter().cloned().collect();
        reversed_phonemes.reverse();
        let reversed = PhonemeInventory::from_phonemes(reversed_phonemes);

        let run = |inv: &PhonemeInventory| -> Vec<String> {
            let generator = RootGenerator::new(inv, &phonotactics).unwrap();
            let mut rng = rng_for(0, RngDomain::Roots);
            generator
                .generate(&mut rng, 50)
                .iter()
                .map(|r| r.written(inv).unwrap())
                .collect()
        };
        assert_ne!(run(&forward), run(&reversed));
    }

    /// The data-free distribution tripwire: isolates a weighted-index change from
    /// an RNG change and from any fixture edit.
    #[test]
    fn the_weighted_index_canary_for_seed_zero_is_frozen() {
        let dist = WeightedIndex::new([30u32, 25, 20, 12, 8, 5]).unwrap();
        let mut rng = rng_for(0, RngDomain::Roots);
        let draws: Vec<usize> = (0..16).map(|_| dist.sample(&mut rng)).collect();
        assert_eq!(draws, [1, 0, 1, 4, 1, 1, 1, 4, 2, 0, 1, 5, 2, 0, 2, 3]);
    }

    #[test]
    fn a_root_renders_in_both_romanisation_and_ipa() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_j", "j", SegmentKind::Consonant).with_romanization("y"),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
        ]);
        let root = Root {
            syllables: vec![Syllable {
                pattern: "CV".to_owned(),
                segments: vec![PhonemeId::new("ph_j"), PhonemeId::new("ph_a")],
            }],
        };
        assert_eq!(root.written(&inventory).unwrap(), "ya");
        assert_eq!(root.ipa(&inventory).unwrap(), "ja");
    }

    #[test]
    fn rendering_a_root_against_the_wrong_inventory_errors_rather_than_dropping_segments() {
        let empty = PhonemeInventory::new();
        let root = Root {
            syllables: vec![Syllable {
                pattern: "V".to_owned(),
                segments: vec![PhonemeId::new("ph_a")],
            }],
        };
        assert!(root.written(&empty).is_err());
    }

    /// Property test (`DESIGN.md` §16.3): over many seeds, never an outside segment.
    #[test]
    fn generated_roots_only_ever_contain_inventory_phonemes() {
        let inventory = inventory();
        let phonotactics = phonotactics();
        let generator = RootGenerator::new(&inventory, &phonotactics).unwrap();
        for seed in 0..1_000u64 {
            let mut rng = rng_for(seed, RngDomain::Roots);
            for root in generator.generate(&mut rng, 20) {
                for id in root.segments() {
                    assert!(inventory.get(id).is_some(), "seed {seed}: {id} escaped");
                }
            }
        }
    }

    /// Catches a phoneme silently excluded from a candidate vector.
    #[test]
    fn every_weighted_phoneme_appears_in_a_large_enough_sample() {
        let inventory = inventory();
        let phonotactics = phonotactics();
        let generator = RootGenerator::new(&inventory, &phonotactics).unwrap();
        let mut rng = rng_for(0, RngDomain::Roots);

        let mut seen = std::collections::BTreeSet::new();
        for root in generator.generate(&mut rng, 5_000) {
            for id in root.segments() {
                seen.insert(id.clone());
            }
        }
        for phoneme in inventory.iter() {
            assert!(seen.contains(&phoneme.id), "{} never appeared", phoneme.ipa);
        }
    }
}
