//! Seeding a proto-lexicon from the built-in concept list (ROADMAP M2).
//!
//! # The lexicon draw-order contract
//!
//! Normative, in the register of `stem_phonology::generate`'s, and pinned by
//! `building_over_fewer_concepts_is_a_prefix_of_building_over_more`. If it drifts,
//! every previously generated lexicon is silently invalidated, and any change to
//! it requires a `PROGRESS.md` entry.
//!
//! One [`StemmaRng`] from `rng_for(seed, RngDomain::Lexicon)`, never re-seeded. For
//! each concept of the slice, **in list order**, exactly one call to
//! `RootGenerator::next_root`. Nothing else draws: the part of speech is copied
//! from the concept, the gloss is left empty, the ids are counters. There is no
//! rejection sampling, no retry, and no draw whose count depends on a previous
//! result.
//!
//! Consequences: `build(n)` is a strict prefix of `build(n + k)` in entries and in
//! draws alike, so `--concepts 25` and `--concepts 100` agree on their first 25
//! words. **Appending** a concept to `CONCEPTS` cannot change any earlier word;
//! **inserting** one anywhere else rewrites every word after it, which is why the
//! list is documented append-only in position.
//!
//! # Homophones are emitted, not prevented
//!
//! Two concepts drawing the same form is expected, and it is real — English has
//! *bark* and *bark*. Rejection sampling would make the draw count depend on the
//! words already produced and destroy the prefix property, which is the same
//! argument that keeps rejection sampling out of M1 entirely.
//! `check_against_inventory` reports them at Note severity and the CLI prints the
//! count to stderr. §17: report, don't police.

use stem_core::RngDomain;
use stem_core::rng::StemmaRng;
use stem_core::{CognateSetId, LanguageId, Result, WordId, rng_for};
use stem_phonology::{PhonemeInventory, Phonotactics, RootGenerator};

use crate::concept::{Concept, ConceptKey};
use crate::lexicon::Lexicon;
use crate::word::{WordEntry, WordSource};

/// Mints the cognate set for the `ordinal`-th entry of `language`'s proto-lexicon:
/// `cog_proto_asterian_0007`.
///
/// # Why the language id is in the string
///
/// A [`CognateSetId`] is **family-scoped**, but nothing in a project file
/// coordinates minting between two independently authored proto-languages. Without
/// a scope, Proto-Asterian's `cog_0001` and Proto-Kelvish's `cog_0001` are the same
/// string, and an M5 table over both would silently assert they are related.
/// Cognacy is a claim about *descent*; scoping the id to the language that coined
/// the word is what keeps the claim true. The cost is one longer column, which is
/// `docs/adr/0003`'s legibility-over-brevity trade — `cog_proto_asterian_0007` says
/// where the thread starts and `cog_0007` requires a lookup.
///
/// # Why it decodes unambiguously
///
/// The ordinal is a pure digit string, so the id splits at the **last** underscore
/// whatever the language id contains. A language called `a_0001` yields
/// `cog_a_0001_0002`, which still decodes to (`a_0001`, 2). The scope is therefore
/// not sanitised: sanitising would be lossy and could collide two languages onto
/// one prefix, which is the very thing the scope exists to prevent.
///
/// **This is the only place in the workspace that mints a `CognateSetId`**, and a
/// test asserts it. M4's fork copies; it never mints.
pub fn scoped_cognate_set(language: &LanguageId, ordinal: usize) -> CognateSetId {
    CognateSetId::new(format!("cog_{language}_{ordinal:04}"))
}

/// Builds a proto-lexicon: one word per concept, in concept-list order.
///
/// Takes an inventory and phonotactics rather than a genome, so `stem_lexicon`
/// stays below `stem_genome` — the same borrow `RootGenerator::new` takes, and for
/// the same reason (`docs/adr/0002`). `language` is needed only to scope the minted
/// [`CognateSetId`]s.
///
/// # Failure
///
/// The body constructs a [`RootGenerator`] and propagates its error **and nothing
/// else**. It deliberately does not call `inventory.validate()`:
/// `RootGenerator::new` filters feature-only codes out of the generation gate so
/// that a half-featured file still generates, and re-validating here would make
/// `new-lexicon` refuse a language `generate-roots` accepts — the validator and
/// the engine disagreeing, which is the defect M1's review caught.
/// `a_language_that_can_generate_roots_can_seed_a_lexicon` exists to stop it.
pub fn build_proto_lexicon(
    language: &LanguageId,
    inventory: &PhonemeInventory,
    phonotactics: &Phonotactics,
    concepts: &[Concept],
    seed: u64,
) -> Result<Lexicon> {
    let generator = RootGenerator::new(inventory, phonotactics)?;
    let mut rng: StemmaRng = rng_for(seed, RngDomain::Lexicon);

    let entries: Vec<WordEntry> = concepts
        .iter()
        .enumerate()
        .map(|(i, concept)| {
            let ordinal = i + 1;
            WordEntry {
                id: WordId::sequential(ordinal),
                concept: Some(ConceptKey::new(concept.key)),
                phonemic_form: generator.next_root(&mut rng),
                glosses: Vec::new(),
                part_of_speech: concept.part_of_speech,
                cognate_set: scoped_cognate_set(language, ordinal),
                source: WordSource::Generated,
                trace: None,
            }
        })
        .collect();

    Ok(Lexicon::from_entries(entries))
}
