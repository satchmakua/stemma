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
//! **M15 extends the contract without weakening it.** A culture profile can remove a
//! meaning or split it into several, and neither may move a word it did not touch:
//! an absent concept still *draws* and discards its root, and elaboration's extra
//! words come from a separate stream. Both clauses are stated on
//! [`build_shaped_lexicon`], which is the one function that implements them —
//! `build_proto_lexicon` is it with an empty profile.
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
use stem_core::{CognateSetId, LanguageId, Result, WordId, rng_for, rng_for_indexed};
use stem_phonology::{PhonemeInventory, Phonotactics, RootGenerator};

use crate::concept::{ConceptKey, Meaning};
use crate::environment::EnvironmentProfile;
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
    meanings: &[Meaning<'_>],
    seed: u64,
) -> Result<Lexicon> {
    build_shaped_lexicon(
        language,
        inventory,
        phonotactics,
        meanings,
        &EnvironmentProfile::default(),
        seed,
    )
}

/// The same, shaped by this culture's [`EnvironmentProfile`] (M15): meanings it says
/// these speakers lack are **not coined**, and meanings it elaborates are coined once
/// per named sense.
///
/// [`build_proto_lexicon`] is this function with an empty profile, so there is one
/// code path and a pre-M15 language cannot be coined by subtly different rules.
///
/// # The shaping must not move a word it did not shape
///
/// This is the whole difficulty, and the module doc's draw contract is extended by
/// exactly two clauses to handle it:
///
/// 1. **Every concept draws from the lexicon stream, in list order, whether or not it
///    is coined.** An absent meaning's root is drawn and *discarded*. Skipping the
///    draw would make every later word's form depend on how many earlier meanings the
///    profile happened to remove, so adding one absence would silently rewrite the
///    rest of the dictionary.
/// 2. **Elaboration's extra words come from [`RngDomain::Elaboration`]**, a separate
///    stream **indexed by the concept's own position**. Taking them from the lexicon
///    stream would shift every word after an elaborated concept, which is the same
///    defect from the other direction — and it is exactly the hazard
///    `RngDomain::Lexicon`'s own docs predicted for "the first time lexicon seeding
///    wants one extra draw per entry". Taking them from a single *shared* elaboration
///    stream is subtler and was the first draft's real bug: one cursor across all
///    elaborations means giving *sand* a fourth word silently moves *water*'s second
///    and third. [`rng_for_indexed`] gives each position its own stream, so a draw
///    depends on what it is for and never on how many draws came before it.
///
/// Together these make a word's form a pure function of its **concept's position**
/// and the seed. So two cultures over one concept list and one seed agree on the form
/// of every meaning they both have, and differ only where their profiles differ —
/// which is what makes M15's acceptance a legible comparison instead of two unrelated
/// languages. Change the seed if you want them to share nothing.
///
/// **Ordinals are still sequential over *coined* words**, unchanged: word ids and
/// cognate sets are per-language identities, and leaving numbered holes where a
/// culture has no word would leak the built-in list's shape into every file.
pub fn build_shaped_lexicon(
    language: &LanguageId,
    inventory: &PhonemeInventory,
    phonotactics: &Phonotactics,
    meanings: &[Meaning<'_>],
    environment: &EnvironmentProfile,
    seed: u64,
) -> Result<Lexicon> {
    let generator = RootGenerator::new(inventory, phonotactics)?;
    let mut rng: StemmaRng = rng_for(seed, RngDomain::Lexicon);

    let shaping = crate::environment::shaping(environment, meanings);
    let mut entries: Vec<WordEntry> = Vec::with_capacity(meanings.len());
    let mut ordinal = 0usize;

    for (position, (meaning, salience)) in meanings.iter().zip(&shaping).enumerate() {
        // Clause 1: the draw happens for every concept, coined or not.
        let root = generator.next_root(&mut rng);
        let words = salience.word_count();
        if words == 0 {
            continue;
        }

        // Clause 2: this concept's OWN elaboration stream, salted with its position,
        // so its extra words depend on nothing any other concept did. Built even
        // when unused (an ordinary meaning draws nothing from it), because
        // constructing it is cheap and branching on the salience here would put the
        // clause in two places.
        let mut elaboration: StemmaRng =
            rng_for_indexed(seed, RngDomain::Elaboration, position as u64);

        // The glosses this meaning is coined under. An elaborated concept replaces
        // its own gloss with the author's named distinctions (see `Elaboration`);
        // otherwise the single word keeps the ordinary rule — a built-in meaning's
        // gloss comes from the compiled table, so storing it would put hundreds of
        // identical strings in every project file, while a **project** meaning has
        // no compiled entry to fall back on and `display_gloss` takes no context.
        // That echo's paired guard is `concepts.stale_project_gloss`.
        for index in 0..words {
            ordinal += 1;
            let form = if index == 0 {
                root.clone()
            } else {
                generator.next_root(&mut elaboration)
            };
            let glosses = match salience {
                crate::environment::Salience::Elaborated(senses) => {
                    vec![senses[index].clone()]
                }
                _ if meaning.is_builtin => Vec::new(),
                _ => vec![meaning.gloss.to_owned()],
            };
            entries.push(WordEntry {
                id: WordId::sequential(ordinal),
                concept: Some(ConceptKey::new(meaning.key)),
                phonemic_form: form,
                glosses,
                part_of_speech: meaning.part_of_speech,
                cognate_set: scoped_cognate_set(language, ordinal),
                source: WordSource::Generated,
                trace: None,
                morphemes: Vec::new(),
                // A coined root is monomorphemic and derived from nothing: it is
                // one draw, not a composition. M14's `derive` is what writes bases.
                bases: Vec::new(),
                senses: Vec::new(),
                sense_history: None,
            });
        }
    }

    Ok(Lexicon::from_entries(entries))
}
