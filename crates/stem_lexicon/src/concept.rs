//! The built-in concept list: what two languages are compared *at*.
//!
//! A concept is a language-neutral meaning. It is the join key that makes two
//! independently generated languages comparable (`DESIGN.md` §10.3), and the
//! anchor a proto-lexicon is built from.
//!
//! It is deliberately **not** a cognate set. A concept is shared *meaning*; a
//! cognate set is shared *ancestry*; and after M9's semantic drift those give
//! different answers — Latin `caput` "head" and French `chef` "chief" share a
//! cognate set and not a concept. CLDF keeps `Parameter_ID` and `Cognateset_ID`
//! in different columns for exactly this reason, and so does Stemma.
//!
//! # Why the table is compiled in
//!
//! `docs/adr/0004`'s argument transfers, with one addition specific to this
//! project. A concept list read from a sidecar data file would make a generated
//! lexicon depend on a file the seed contract does not cover —
//! [`stem_genome::LanguageGenome::seed`] promises a result is reproducible *from
//! the file alone*, and `new-lexicon --seed 42` producing different languages on
//! two machines with different concept files breaks that outright. It is the same
//! hazard that made `Phonotactics::default()` empty rather than `(C)V(C)`.
//!
//! What does *not* transfer from ADR-0004: a feature is load-bearing on the engine
//! (a rule keyed on an unknown feature matches nothing, silently), whereas a
//! concept is inert — nothing in M2, M3 or M4 computes over its meaning. So the
//! set is closed for the *typo* argument only, and an unrecognised key is a
//! **Warning with a suggestion**, not an Error. A conlanger who writes
//! `concept: "OBSIDIAN"` and supplies a gloss gets a working dictionary and a
//! note. That is `CLAUDE.md`'s "report, don't police" applied where it belongs.

use serde::{Deserialize, Serialize};

/// A concept's local, human-readable key: `NOSE`, `SMOKE_EXHAUST`.
///
/// Derived from the Concepticon gloss by uppercasing and replacing every
/// non-alphanumeric run with `_`, which keeps it inside CLDF's `[a-zA-Z0-9_-]+`
/// charset — so a future CLDF export is a rename, not a redesign. This is what
/// appears in RON and in the CSV `Parameter_ID` column, per `docs/adr/0003`:
/// `concept: "NOSE"` is legible, `concept: 1221` is not.
///
/// A foreign key into a compiled table, not an entity id, which is why it is
/// declared here rather than minted by `stem_core`'s `define_id!` — `stem_core`
/// must not learn that meanings exist (`docs/adr/0002`).
///
/// **Keys are permanent.** A key is written into every stored lexicon and is the
/// label of every M5 cognate-table row, so renaming one silently orphans every
/// entry that used it. Concepticon's own answer is `REPLACEMENT_ID`: ids are never
/// reused, and a retired concept keeps its row behind a forwarding pointer. M2
/// adopts the discipline rather than the mechanism — **a wrong key is corrected by
/// appending, never by renaming** — and the mechanism arrives if a key is ever
/// actually retired, which for a frozen 1955 list is close to impossible.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConceptKey(String);

impl ConceptKey {
    /// Wraps a key string.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// The underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the key is empty, which is never meaningful.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for ConceptKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ConceptKey {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for ConceptKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for ConceptKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A word's grammatical class **in one language**.
///
/// Concepticon deliberately assigns no part of speech, because whether `ROUND` is
/// an adjective or a stative verb is a fact about a language rather than about a
/// meaning. So the class lives on the [`crate::WordEntry`]; [`Concept::part_of_speech`]
/// is only the *default* a generated entry starts with, and an author may change
/// it freely. `validate` says nothing about it — §17's posture applied to grammar:
/// the tool guides, it does not police.
///
/// Closed, for `docs/adr/0004`'s reason: `part_of_speech: "nuon"` must be a load
/// error rather than a silently registered category no dictionary section prints.
/// `#[non_exhaustive]` so that appending `Classifier` is not a breaking change.
/// Nothing on disk refers to a variant by index, so appending cannot change what
/// any stored file means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PartOfSpeech {
    /// A thing.
    Noun,
    /// An action or state.
    Verb,
    /// A property.
    Adjective,
    /// A modifier of a verb, adjective, or clause.
    Adverb,
    /// A pro-form standing for a participant.
    Pronoun,
    /// A number word.
    Numeral,
    /// An article, demonstrative, or quantifier.
    Determiner,
    /// A preposition or postposition.
    Adposition,
    /// A function word that fits nowhere else.
    Particle,
}

impl PartOfSpeech {
    /// `Noun` — the serde default for a morpheme's part of speech (M8): nouns
    /// inflect for number in the reference paradigm, and most stems are nouns.
    pub fn noun_default() -> Self {
        Self::Noun
    }

    /// The lowercase name, as it appears in RON and in exports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Noun => "noun",
            Self::Verb => "verb",
            Self::Adjective => "adjective",
            Self::Adverb => "adverb",
            Self::Pronoun => "pronoun",
            Self::Numeral => "numeral",
            Self::Determiner => "determiner",
            Self::Adposition => "adposition",
            Self::Particle => "particle",
        }
    }
}

impl std::fmt::Display for PartOfSpeech {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One meaning on the built-in comparison list.
///
/// Four fields. `semantic_field`, `ontological_category` and `definition` were
/// considered and cut: none has a consumer in M2, each would be ~100 more
/// hand-copied values in the milestone whose largest undetectable-error surface is
/// exactly that, and **a `Concept` never reaches disk** — only its `key` does. So
/// adding them at M6, where a CLDF export gives them a job and a reason to verify
/// them, is invisible to every file saved before then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Concept {
    /// Local identity, CLDF-safe charset, permanent.
    pub key: &'static str,

    /// The permanent external anchor, or `None` where Stemma has no verified
    /// mapping.
    ///
    /// Concepticon never reuses an id and retires one only behind a
    /// `REPLACEMENT_ID` forwarding pointer, so an integer stored in a project file
    /// resolves forever: `1221` is <https://concepticon.clld.org/parameters/1221>.
    /// That permanence is the whole reason the field exists — M5 joins languages
    /// by meaning, and a gloss string is not a stable key. ASJP-40 glosses `path`
    /// as PATH (2252) while Swadesh glosses `road (path)` as ROAD (667): the same
    /// English word, two different concept sets. String matching merges them; the
    /// integer does not.
    ///
    /// **`Option`, and the `None`s are load-bearing.** Two of the three Stemma
    /// additions below had no anchor that could be verified against the Concepticon
    /// data when this table was written. A plausible-looking integer under a column
    /// bearing an external authority's name is a false provenance claim, in a
    /// program whose entire premise is honest provenance. `None` is the true value,
    /// and it is what CLDF expects — `Concepticon_ID` is an optional column.
    pub concepticon_id: Option<u32>,

    /// The English label printed in the dictionary.
    pub gloss: &'static str,

    /// The part of speech a generated entry starts with. A default, not a truth —
    /// see [`PartOfSpeech`].
    pub part_of_speech: PartOfSpeech,
}

/// A meaning **this project declares**, alongside the built-in list (ROADMAP M12).
///
/// # Why a second type rather than a bigger [`CONCEPTS`]
///
/// [`Concept`] is `&'static str` all the way down because it is a compiled table; it
/// cannot be deserialised from a file. This is its owned twin, stored **inside the
/// genome** — which is the point. `concept.rs`'s own docs explain why the built-in
/// list is compiled in: a lexicon generated from a *sidecar* file would depend on
/// something [`stem_genome::LanguageGenome::seed`]'s contract does not cover, and
/// `new-lexicon --seed 42` would produce different languages on two machines. A
/// concept carried in the genome has no such problem — the language is still
/// reproducible from its own file alone, which is why this is a genome field and
/// must never become a `--concepts-file` flag.
///
/// # No `concepticon_id`, deliberately
///
/// A project concept has no external anchor by definition: if a verified Concepticon
/// mapping existed, the meaning belongs on the built-in list where every reader gets
/// it. Omitting the field entirely makes a fabricated anchor unrepresentable rather
/// than merely discouraged — the strongest form of [`Concept::concepticon_id`]'s
/// false-provenance rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConcept {
    /// Local identity, in the same CLDF-safe charset as a built-in key.
    ///
    /// **Permanent, and unique against the built-in list.** A key that collides with
    /// a compiled one is reported (`concepts.shadows_builtin`) rather than silently
    /// winning, because two meanings under one key would make the join ambiguous.
    pub key: ConceptKey,

    /// The English label printed in the dictionary.
    pub gloss: String,

    /// The part of speech a generated entry starts with. A default, not a truth —
    /// see [`PartOfSpeech`].
    #[serde(default = "PartOfSpeech::noun_default")]
    pub part_of_speech: PartOfSpeech,

    /// Authorial prose: why this language needs this meaning. Not interpreted.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// One meaning to coin a word for, from either source.
///
/// The generator needs exactly two things — a key to record and a part of speech to
/// seed — plus a gloss to write onto the entry when the built-in list cannot supply
/// one. Borrowing both sources into one view keeps [`crate::build_proto_lexicon`] a
/// single code path, so a project concept and a built-in concept cannot be coined by
/// subtly different rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meaning<'a> {
    /// The concept key recorded on the entry.
    pub key: &'a str,
    /// Its gloss.
    pub gloss: &'a str,
    /// The part of speech the coined word starts with.
    pub part_of_speech: PartOfSpeech,
    /// True when [`concept`] can resolve `key`, i.e. the gloss is already available
    /// from the compiled table and need not be written onto the entry.
    pub is_builtin: bool,
}

impl<'a> From<&'a Concept> for Meaning<'a> {
    fn from(c: &'a Concept) -> Self {
        Self {
            key: c.key,
            gloss: c.gloss,
            part_of_speech: c.part_of_speech,
            is_builtin: true,
        }
    }
}

impl<'a> From<&'a ProjectConcept> for Meaning<'a> {
    fn from(c: &'a ProjectConcept) -> Self {
        Self {
            key: c.key.as_str(),
            gloss: &c.gloss,
            part_of_speech: c.part_of_speech,
            is_builtin: false,
        }
    }
}

/// Every meaning available to a language: the built-in list, then the project's own,
/// in that order.
///
/// **Built-in first, and that is load-bearing.** `build(n)` must stay a strict prefix
/// of `build(n + k)` ([`crate::build`]'s draw contract), so appending the project's
/// concepts after the compiled ones means declaring a new meaning cannot change a
/// single word that was already coined.
pub fn meanings<'a>(project: &'a [ProjectConcept]) -> Vec<Meaning<'a>> {
    CONCEPTS
        .iter()
        .map(Meaning::from)
        .chain(project.iter().map(Meaning::from))
        .collect()
}

/// Builds a concept carrying a verified Concepticon anchor.
const fn c(
    key: &'static str,
    concepticon_id: u32,
    gloss: &'static str,
    part_of_speech: PartOfSpeech,
) -> Concept {
    Concept {
        key,
        concepticon_id: Some(concepticon_id),
        gloss,
        part_of_speech,
    }
}

/// Builds one of Stemma's own concepts, with no external anchor.
const fn stemma(key: &'static str, gloss: &'static str, part_of_speech: PartOfSpeech) -> Concept {
    Concept {
        key,
        concepticon_id: None,
        gloss,
        part_of_speech,
    }
}

use PartOfSpeech::{Adjective, Determiner, Noun, Numeral, Particle, Pronoun, Verb};

/// The built-in comparison list: the Swadesh 1955 hundred, plus three of Stemma's
/// own.
///
/// # Provenance
///
/// The hundred were taken from Concepticon's `Swadesh-1955-100` conceptlist
/// (Concepticon 3.4.0, CC-BY-4.0), not transcribed from a prose page. Two values
/// that circulate in secondary sources are wrong and the fetched ones are used
/// here: `hair` is **1036** (not 1040) and `root` is **668** (not 670).
///
/// Swadesh-207 was rejected: it is not Swadesh's own list, has no single
/// authoritative publication, and carries items (`snow`, `ice`, `freeze`, `sea`)
/// that a generator would have to coin a word for in *every* language — inventing
/// an obligatory word for ice in a desert proto-language is the tool making a
/// claim rather than reporting one. ASJP-40 was rejected as too thin to read as a
/// dictionary, which is M2's actual deliverable.
///
/// # The three additions
///
/// Not taste. The rule is: *a meaning named by a Phase-1 acceptance test or a
/// `DESIGN.md` worked example must be representable.* Applied, it yields exactly
/// three. `KING` and `MOTHER` are named by ROADMAP M5's own command
/// (`stemma cognates --meanings water sun star king mother`) and neither is on any
/// Swadesh list; `STORM` is named by §21's demo, which turns on the river people's
/// word for *king* being cognate with the mountain people's word for *storm*.
/// Without them M5 cannot run its own acceptance test.
///
/// # Order is part of the determinism contract
///
/// Normative. This is the order roots are drawn in ([`crate::build`]), the order
/// entries are stored, and the order they are exported. **Appending is safe and is
/// how the list grows; inserting anywhere else rewrites every word after the
/// insertion point in every lexicon ever generated.** Same contract as authored
/// inventory order, and it is why the three additions sit at the end rather than
/// in alphabetical position.
pub const CONCEPTS: &[Concept] = &[
    c("ALL", 98, "all", Determiner),
    c("ASH", 646, "ashes", Noun),
    c("BARK", 1204, "bark", Noun),
    c("BELLY", 1251, "belly", Noun),
    c("BIG", 1202, "big", Adjective),
    c("BIRD", 937, "bird", Noun),
    c("BITE", 1403, "bite", Verb),
    c("BLACK", 163, "black", Adjective),
    c("BLOOD", 946, "blood", Noun),
    c("BONE", 1394, "bone", Noun),
    c("BREAST", 1402, "breast", Noun),
    c("BURN", 2102, "burn", Verb),
    c("CLAW", 72, "claw", Noun),
    c("CLOUD", 1489, "cloud", Noun),
    c("COLD", 1287, "cold", Adjective),
    c("COME", 1446, "come", Verb),
    c("DIE", 1494, "die", Verb),
    c("DOG", 2009, "dog", Noun),
    c("DRINK", 1401, "drink", Verb),
    c("DRY", 1398, "dry", Adjective),
    c("EAR", 1247, "ear", Noun),
    c("EARTH_SOIL", 1228, "earth", Noun),
    c("EAT", 1336, "eat", Verb),
    c("EGG", 744, "egg", Noun),
    c("EYE", 1248, "eye", Noun),
    c("FAT_ORGANIC_SUBSTANCE", 323, "fat (grease)", Noun),
    c("FEATHER", 1201, "feather", Noun),
    c("FIRE", 221, "fire", Noun),
    c("FISH", 227, "fish", Noun),
    c("FLY_MOVE_THROUGH_AIR", 1441, "fly", Verb),
    c("FOOT", 1301, "foot", Noun),
    c("FULL", 1429, "full", Adjective),
    c("GIVE", 1447, "give", Verb),
    c("GOOD", 1035, "good", Adjective),
    c("GREEN", 1425, "green", Adjective),
    c("HAIR", 1036, "hair", Noun),
    c("HAND", 1277, "hand", Noun),
    c("HEAD", 1256, "head", Noun),
    c("HEAR", 1408, "hear", Verb),
    c("HEART", 1223, "heart", Noun),
    c("HORN_ANATOMY", 1393, "horn", Noun),
    c("I", 1209, "I", Pronoun),
    c("KILL", 1417, "kill", Verb),
    c("KNEE", 1371, "knee", Noun),
    c("KNOW_SOMETHING", 1410, "know", Verb),
    c("LEAF", 628, "leaf", Noun),
    c("LIE_REST", 1411, "lie", Verb),
    c("LIVER", 1224, "liver", Noun),
    c("LONG", 1203, "long", Adjective),
    c("LOUSE", 1392, "louse", Noun),
    c("MAN", 1554, "man", Noun),
    c("MANY", 1198, "many", Determiner),
    c("FLESH_OR_MEAT", 2615, "meat (flesh)", Noun),
    c("MOON", 1313, "moon", Noun),
    c("MOUNTAIN", 639, "mountain", Noun),
    c("MOUTH", 674, "mouth", Noun),
    c("NAME", 1405, "name", Noun),
    c("NECK", 1333, "neck", Noun),
    c("NEW", 1231, "new", Adjective),
    c("NIGHT", 1233, "night", Noun),
    c("NOSE", 1221, "nose", Noun),
    c("NOT", 1240, "not", Particle),
    c("ONE", 1493, "one", Numeral),
    c("PERSON", 683, "person (human being)", Noun),
    c("RAINING_OR_RAIN", 2108, "rain", Noun),
    c("RED", 156, "red", Adjective),
    c("ROAD", 667, "road (path)", Noun),
    c("ROOT", 668, "root", Noun),
    c("ROUND", 1395, "round", Adjective),
    c("SAND", 670, "sand", Noun),
    c("SAY", 1458, "say", Verb),
    c("SEE", 1409, "see", Verb),
    c("SEED", 714, "seed", Noun),
    c("SIT", 1416, "sit", Verb),
    c("SKIN", 763, "skin", Noun),
    c("SLEEP", 1585, "sleep", Verb),
    c("SMALL", 1246, "small", Adjective),
    c("SMOKE_EXHAUST", 778, "smoke", Noun),
    c("STAND", 1442, "stand", Verb),
    c("STAR", 1430, "star", Noun),
    c("STONE", 857, "stone", Noun),
    c("SUN", 1343, "sun", Noun),
    c("SWIM", 1439, "swim", Verb),
    c("TAIL", 1220, "tail", Noun),
    c("THAT", 78, "that", Determiner),
    c("THIS", 1214, "this", Determiner),
    c("THOU", 1215, "thou", Pronoun),
    c("TONGUE", 1205, "tongue", Noun),
    c("TOOTH", 1380, "tooth", Noun),
    c("TREE", 906, "tree", Noun),
    c("TWO", 1498, "two", Numeral),
    c("WALK", 1443, "walk", Verb),
    c("HOT_OR_WARM", 2272, "warm (hot)", Adjective),
    c("WATER", 948, "water", Noun),
    c("WE", 1212, "we", Pronoun),
    c("WHAT", 1236, "what", Pronoun),
    c("WHITE", 1335, "white", Adjective),
    c("WHO", 1235, "who", Pronoun),
    c("WOMAN", 962, "woman", Noun),
    c("YELLOW", 1424, "yellow", Adjective),
    // --- Stemma additions. Not on any Swadesh list; each is named by a Phase-1
    //     acceptance test or a DESIGN worked example (see the docs above).
    //     Appended, never interleaved: the first 100 draws must not move.
    c("MOTHER", 1216, "mother", Noun),
    stemma("KING", "king", Noun),
    stemma("STORM", "storm", Noun),
];

/// How many concepts the built-in list holds.
///
/// `const` so the CLI's `--concepts` bound is derived rather than restated.
pub const CONCEPT_COUNT: usize = CONCEPTS.len();

/// How many of [`CONCEPTS`] come from the Swadesh 1955 list, and therefore carry a
/// verified Concepticon anchor.
pub const SWADESH_COUNT: usize = 100;

/// Resolves a key against the built-in list.
///
/// Linear over ~100 `&'static str` comparisons. A map would be no faster at this
/// size, and is forbidden anyway on any path that can reach an export.
pub fn concept(key: &ConceptKey) -> Option<&'static Concept> {
    CONCEPTS.iter().find(|c| c.key == key.as_str())
}

/// The nearest real key to a typo, for the `unknown_concept` diagnostic.
pub fn nearest_concept_key(key: &ConceptKey) -> Option<&'static str> {
    stem_core::suggest::nearest(key.as_str(), CONCEPTS.iter().map(|c| c.key))
}

/// The concept whose gloss is exactly `gloss`, matched case-insensitively.
///
/// The meaning→*concept* resolver, for diagnostics and (later) CLDF anchoring.
/// It is **not** M5's cognate-table join: that anchor is
/// [`crate::Lexicon::by_meaning`] → `cognate_set`, which matches a word's
/// *displayed* gloss so a gloss override like `king` on concept MAN resolves to
/// the word rather than to the (word-less) KING concept. Well-defined because
/// `the_gloss_column_is_unique` asserts no two concepts share one gloss.
pub fn concept_by_gloss(gloss: &str) -> Option<&'static Concept> {
    CONCEPTS
        .iter()
        .find(|c| c.gloss.eq_ignore_ascii_case(gloss))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn the_concept_list_has_one_hundred_and_three_unique_keys() {
        assert_eq!(CONCEPT_COUNT, 103);
        let keys: BTreeSet<&str> = CONCEPTS.iter().map(|c| c.key).collect();
        assert_eq!(keys.len(), CONCEPT_COUNT, "a concept key is duplicated");
    }

    /// Keys reach the CSV `Parameter_ID` column, so they must stay inside CLDF's
    /// charset or a future CLDF export becomes a redesign rather than a rename.
    #[test]
    fn every_concept_key_is_cldf_safe() {
        for concept in CONCEPTS {
            assert!(
                !concept.key.is_empty()
                    && concept
                        .key
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                "`{}` is not [A-Z0-9_]+",
                concept.key
            );
        }
    }

    #[test]
    fn every_present_concepticon_id_is_unique_and_nonzero() {
        let ids: Vec<u32> = CONCEPTS.iter().filter_map(|c| c.concepticon_id).collect();
        let unique: BTreeSet<u32> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "a Concepticon id is duplicated");
        assert!(!unique.contains(&0), "0 is not a Concepticon id");
    }

    /// The M5 join key must be well-defined: `--meanings star` has to resolve to
    /// exactly one concept.
    #[test]
    fn the_gloss_column_is_unique() {
        let glosses: BTreeSet<String> = CONCEPTS.iter().map(|c| c.gloss.to_lowercase()).collect();
        assert_eq!(glosses.len(), CONCEPT_COUNT, "two concepts share a gloss");
    }

    #[test]
    fn every_swadesh_concept_carries_a_concepticon_anchor() {
        for concept in &CONCEPTS[..SWADESH_COUNT] {
            assert!(
                concept.concepticon_id.is_some(),
                "`{}` came from Swadesh-1955-100 and must carry its anchor",
                concept.key
            );
        }
    }

    /// The test that would have caught a Swadesh-only list. ROADMAP M5's own
    /// acceptance command names five meanings; two are not on any Swadesh list.
    #[test]
    fn the_concepts_roadmap_m5_names_are_all_present() {
        for gloss in ["water", "sun", "star", "king", "mother"] {
            assert!(
                concept_by_gloss(gloss).is_some(),
                "`stemma cognates --meanings … {gloss} …` could not resolve `{gloss}`"
            );
        }
    }

    #[test]
    fn the_stemma_additions_sit_after_the_swadesh_hundred() {
        // Appending is what preserves the prefix property of the draw contract.
        let tail: Vec<&str> = CONCEPTS[SWADESH_COUNT..].iter().map(|c| c.key).collect();
        assert_eq!(tail, ["MOTHER", "KING", "STORM"]);
    }

    #[test]
    fn a_key_resolves_to_its_concept() {
        let star = concept(&ConceptKey::new("STAR")).expect("STAR is on the list");
        assert_eq!(star.gloss, "star");
        assert_eq!(star.part_of_speech, PartOfSpeech::Noun);
        assert_eq!(star.concepticon_id, Some(1430));
    }

    #[test]
    fn an_unknown_key_resolves_to_nothing_and_suggests_the_nearest() {
        let typo = ConceptKey::new("NOES");
        assert!(concept(&typo).is_none());
        assert_eq!(nearest_concept_key(&typo), Some("NOSE"));
    }

    #[test]
    fn a_wildly_wrong_key_suggests_nothing() {
        assert_eq!(nearest_concept_key(&ConceptKey::new("OBSIDIAN")), None);
    }

    #[test]
    fn a_concept_key_round_trips_as_a_bare_string() {
        let key = ConceptKey::new("SMOKE_EXHAUST");
        let json = serde_json::to_string(&key).expect("serialise");
        assert_eq!(json, "\"SMOKE_EXHAUST\"");
        assert_eq!(
            serde_json::from_str::<ConceptKey>(&json).expect("deserialise"),
            key
        );
    }

    #[test]
    fn a_part_of_speech_round_trips_in_snake_case() {
        let json = serde_json::to_string(&PartOfSpeech::Determiner).expect("serialise");
        assert_eq!(json, "\"determiner\"");
        assert_eq!(PartOfSpeech::Determiner.name(), "determiner");
    }
}
