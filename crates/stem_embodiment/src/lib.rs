//! What a speaker **is**, and which of Stemma's machinery applies to it
//! (ROADMAP M23, `DESIGN.md` §18.1–§18.2, §7.7).
//!
//! # Alien is not "impossible sounds"
//!
//! §18's opening line: *alien language support should not mean "random impossible
//! sounds". It should mean communication systems shaped by alien bodies and
//! environments.* So this crate models the **body** — the channels it can signal on
//! and the constraints those channels carry — and says nothing about signals
//! themselves. Generalising a phoneme to a channel signal is M24's job, and doing it
//! here would be building the interesting half before the boring half is honest.
//!
//! # The load-bearing half is the report, not the struct
//!
//! ROADMAP M23's acceptance: *a profile with no vocal tract but a bioluminescent
//! channel validates, and the engine reports which existing machinery does **not**
//! apply to it rather than silently producing vowels.*
//!
//! Almost every layer below `stem_genome` assumes a vocal tract. A `PhonemeInventory`
//! is a set of things a mouth can do; a `CVC` template describes a syllable; stress is
//! a property of a syllable; the whole sound-change engine transforms feature bundles
//! that name a tongue and a larynx. None of that is *wrong* — it is simply about a body
//! this speaker does not have, and the failure mode to avoid is the tool carrying on
//! regardless and handing a bioluminescent species a five-vowel system.
//!
//! So [`applicability`] answers the question subsystem by subsystem, in the tool's own
//! words, and [`VOCAL_TRACT_CHECKS`] names the validation checks that mean nothing for
//! a body with no mouth — the same list, made operational. M0 wrote the first half of
//! this sentence three years of milestones ago: `no_nucleus`'s message already ends
//! *"(a non-vocal language will need the alien modality model of §7.7)"*.
//!
//! # One `Vec<Channel>`, not five
//!
//! §18.1 sketches `visual_channels`, `chemical_channels`, `tactile_channels` and
//! `electric_or_magnetic_channels` as four parallel `Vec`s. This crate carries **one**
//! `Vec<Channel>` whose members name their own [`ChannelKind`], and the deviation is
//! deliberate:
//!
//! - §18.2 gives every channel the *same* ten constraints, so the four structs would
//!   have been identical but for the pile they sat in;
//! - the medium is a property of the channel, not a filing decision made by whoever
//!   typed the file;
//! - four parallel lists is the shape that guarantees a renderer eventually iterates
//!   three of them.
//!
//! §18.1's `environment: EnvironmentProfile` is kept exactly as written — M15's
//! ecology work is the precursor the roadmap says it is, not a detour.

use serde::{Deserialize, Serialize};
use stem_core::{Issue, Severity, Validate, ValidationReport};

// ------------------------------------------------------------------ constraints

/// An authored judgement on a scale, in bands.
///
/// §18.2 asks for bandwidth, range, energy cost, learnability, noise and cultural
/// salience. None of those is measurable from anything in a language file — they are
/// the author's claims about a species — so they are bands rather than numbers, for the
/// reason every other scale in this project is (`docs/adr/0009`). A number here would
/// be a fabricated measurement of an invented creature, which is worse than a
/// fabricated measurement of a real one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Magnitude {
    /// Not stated. Prints as `—`; never silently read as `Low`.
    #[default]
    Unspecified,
    Low,
    Moderate,
    High,
}

impl Magnitude {
    /// Its name, for the sketch. `Unspecified` is a dash, never the commonest value.
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "—",
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
        }
    }
}

/// How long a signal lasts once it has been made.
///
/// The constraint with the largest grammatical consequences (§18.3): a scent that
/// lingers can be *found later*, which is a different communicative act from a sound
/// that is gone the moment it is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Persistence {
    #[default]
    Unspecified,
    /// Gone as it is made — speech, a light pulse.
    Fleeting,
    /// Fades over minutes or hours.
    Lingering,
    /// Stays until something removes it — a scent mark, a deposited trace.
    Persistent,
}

impl Persistence {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "—",
            Self::Fleeting => "fleeting",
            Self::Lingering => "lingering",
            Self::Persistent => "persistent",
        }
    }
}

/// Whether the channel can carry more than one parameter at once.
///
/// Human speech is overwhelmingly sequential; a chromatophore display is not, and
/// §18.3 draws simultaneous morphology straight out of that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Simultaneity {
    #[default]
    Unspecified,
    /// One thing at a time.
    Sequential,
    /// Several independent parameters at once — colour, rhythm and position together.
    Simultaneous,
}

impl Simultaneity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "—",
            Self::Sequential => "sequential",
            Self::Simultaneous => "simultaneous",
        }
    }
}

/// Who can receive a signal, and from where.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Directionality {
    #[default]
    Unspecified,
    /// Radiates to everyone in range.
    Omnidirectional,
    /// Aimed, and received only by those in the beam.
    Directional,
    /// Requires touch or near-touch — an electric field, a gesture read by hand.
    Contact,
}

impl Directionality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "—",
            Self::Omnidirectional => "omnidirectional",
            Self::Directional => "directional",
            Self::Contact => "contact",
        }
    }
}

/// Whether a bystander in range necessarily receives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Privacy {
    #[default]
    Unspecified,
    /// Everyone in range receives it whether addressed or not.
    Broadcast,
    /// Reaches the addressee and not the neighbours.
    Targeted,
}

impl Privacy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "—",
            Self::Broadcast => "broadcast",
            Self::Targeted => "targeted",
        }
    }
}

// --------------------------------------------------------------------- channels

/// The medium a channel signals in — §7.7's list.
///
/// `Vocal` is on it deliberately. Human speech is one channel among these rather than
/// the default the others deviate from, and a model in which the ordinary case is
/// unnamed is one that will keep treating it as unmarked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ChannelKind {
    #[default]
    Unspecified,
    /// A vocal tract moving air — everything Stemma models today.
    Vocal,
    /// Pulses above the speaker's own hearing range.
    Ultrasonic,
    /// Low-frequency resonance, felt as much as heard.
    Infrasonic,
    /// Emitted light, in bands.
    Bioluminescent,
    /// Skin patterning — colour and texture over an area.
    Chromatophore,
    /// Limb position and movement.
    Gesture,
    /// Modulated electric or magnetic fields.
    FieldPulse,
    /// Released chemical packets.
    Chemical,
    /// Pressure waves through a liquid or a solid.
    PressureWave,
    /// Several speakers producing one utterance together.
    HiveHarmonic,
}

impl ChannelKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Vocal => "vocal",
            Self::Ultrasonic => "ultrasonic",
            Self::Infrasonic => "infrasonic",
            Self::Bioluminescent => "bioluminescent",
            Self::Chromatophore => "chromatophore",
            Self::Gesture => "gesture",
            Self::FieldPulse => "field pulse",
            Self::Chemical => "chemical",
            Self::PressureWave => "pressure wave",
            Self::HiveHarmonic => "hive harmonic",
        }
    }

    /// Whether this channel is produced by a vocal tract.
    ///
    /// The one question the kind is consulted for — [`ScriptKind::expects_unwritten`]'s
    /// discipline. Everything else about a channel is declared, constraint by
    /// constraint, rather than inferred from its medium.
    ///
    /// [`ScriptKind::expects_unwritten`]: https://docs.rs/stem_script
    pub fn is_vocal(self) -> bool {
        matches!(self, Self::Vocal)
    }
}

/// One channel a species can signal on, with §18.2's constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    /// Stable id within this profile.
    pub id: String,
    /// Display name: `"the mantle bands"`.
    pub name: String,
    /// The medium.
    #[serde(default)]
    pub kind: ChannelKind,

    // --- §18.2's constraints. The four below drive reported consequences (§18.3);
    //     the six after them are printed and not otherwise interpreted, which the
    //     sketch says plainly rather than implying they are inert.
    /// How long a signal lasts.
    #[serde(default)]
    pub persistence: Persistence,
    /// Whether several parameters can ride at once.
    #[serde(default)]
    pub simultaneity: Simultaneity,
    /// Who receives it, and from where.
    #[serde(default)]
    pub directionality: Directionality,
    /// Whether bystanders receive it too.
    #[serde(default)]
    pub privacy: Privacy,

    /// How much can be said per unit time.
    #[serde(default)]
    pub bandwidth: Magnitude,
    /// How far it carries.
    #[serde(default)]
    pub range: Magnitude,
    /// What it costs the speaker to produce.
    #[serde(default)]
    pub energy_cost: Magnitude,
    /// How hard it is to acquire.
    #[serde(default)]
    pub learnability: Magnitude,
    /// How much the environment interferes.
    #[serde(default)]
    pub noise: Magnitude,
    /// How much the culture invests in it.
    #[serde(default)]
    pub cultural_salience: Magnitude,

    /// Authorial prose. Not interpreted.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

// ------------------------------------------------------------------ the body

/// A vocal tract.
///
/// **Deliberately almost empty**, and the emptiness is the design. A language with a
/// vocal tract already describes it in exhaustive detail — that is what
/// `PhonemeInventory` and its feature bundles *are*. A second description here would be
/// two sources of truth about one anatomy, which is the desynchronisation M2 refused
/// when it would not store a rendered string beside `phonemic_form`.
///
/// What matters is that the field is an `Option` on [`EmbodimentProfile`]. `None` is
/// the whole alien case.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VocalTractProfile {
    /// Authorial prose: what the anatomy is like, where it differs from a human one.
    /// Not interpreted — the inventory is the model.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// A limb or organ that can be positioned to mean something.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManipulatorProfile {
    /// Display name: `"the feeding arms"`.
    pub name: String,
    /// How many there are. Drives §18.3's poly-manual agreement consequence, which is
    /// why it is a count and not prose.
    #[serde(default)]
    pub count: u32,
    /// How finely it can be controlled.
    #[serde(default)]
    pub dexterity: Magnitude,
    /// Authorial prose.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// How the speakers are organised as minds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SocialStructure {
    #[default]
    Unspecified,
    /// Individuals who speak one at a time — the human case.
    Individual,
    /// Standing groups within which a message is shared.
    Collective,
    /// One utterance requires several speakers; the individual is not the unit.
    Hive,
}

impl SocialStructure {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "—",
            Self::Individual => "individual",
            Self::Collective => "collective",
            Self::Hive => "hive",
        }
    }
}

/// §18.1's social cognition profile.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialCognitionProfile {
    /// How the speakers are organised.
    #[serde(default)]
    pub structure: SocialStructure,
    /// How many speakers a complete utterance needs. `1` is the ordinary case; a hive
    /// harmonic language may need more, which §18.2 calls the simultaneity constraint
    /// at the level of the *speaker* rather than the signal.
    #[serde(default)]
    pub speakers_per_utterance: u32,
    /// Authorial prose.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl SocialCognitionProfile {
    /// Whether anything about the social structure has been stated.
    pub fn is_empty(&self) -> bool {
        self.structure == SocialStructure::Unspecified
            && self.speakers_per_utterance == 0
            && self.note.is_empty()
    }
}

/// §18.1's embodiment profile: what a speaker is.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbodimentProfile {
    /// The vocal tract, if there is one. **`None` is the alien case**, and the field
    /// every applicability answer turns on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vocal_tract: Option<VocalTractProfile>,
    /// Every channel this species can signal on, in authored order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<Channel>,
    /// Limbs or organs that can be positioned to mean something.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manipulators: Vec<ManipulatorProfile>,
    /// How the speakers are organised as minds.
    #[serde(default)]
    pub social_cognition: SocialCognitionProfile,
    /// §18.1's own field, and M15's type unchanged: the ecology these speakers live in.
    #[serde(default)]
    pub environment: stem_lexicon::EnvironmentProfile,
    /// Authorial prose: what this creature is.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl EmbodimentProfile {
    /// Whether anything at all has been declared.
    ///
    /// A language with no profile is the ordinary case and must not be treated as an
    /// alien one — every fixture in this project has speakers with mouths, and none of
    /// them says so.
    pub fn is_empty(&self) -> bool {
        self.vocal_tract.is_none()
            && self.channels.is_empty()
            && self.manipulators.is_empty()
            && self.social_cognition.is_empty()
            && self.environment.is_empty()
            && self.note.is_empty()
    }

    /// Whether this speaker has a vocal tract, or has said nothing about it.
    ///
    /// **An empty profile answers `true`.** Silence is not a claim to be alien, and
    /// reading it as one would set aside the vocal-tract checks for every language ever
    /// written in this project.
    pub fn has_vocal_tract(&self) -> bool {
        self.is_empty()
            || self.vocal_tract.is_some()
            || self.channels.iter().any(|c| c.kind.is_vocal())
    }

    /// The channels that are not a vocal tract, in authored order.
    pub fn non_vocal_channels(&self) -> Vec<&Channel> {
        self.channels
            .iter()
            .filter(|c| !c.kind.is_vocal())
            .collect()
    }
}

// ------------------------------------------------------------- applicability

/// A part of Stemma that a language may or may not be able to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Subsystem {
    Phonology,
    Phonotactics,
    Prosody,
    SoundChange,
    Morphology,
    Semantics,
    Syntax,
    Script,
}

impl Subsystem {
    pub fn label(self) -> &'static str {
        match self {
            Self::Phonology => "Phonology",
            Self::Phonotactics => "Phonotactics",
            Self::Prosody => "Prosody",
            Self::SoundChange => "Sound change",
            Self::Morphology => "Morphology",
            Self::Semantics => "Semantics",
            Self::Syntax => "Syntax",
            Self::Script => "Script",
        }
    }
}

/// Whether a subsystem means anything for a given body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Applies {
    /// Works as documented.
    Yes,
    /// Works, but means something other than what its name suggests.
    Partly,
    /// Assumes anatomy this speaker does not have.
    No,
    /// Needs machinery that is not built yet.
    Unbuilt,
}

impl Applies {
    pub fn label(self) -> &'static str {
        match self {
            Self::Yes => "applies",
            Self::Partly => "partly",
            Self::No => "does not apply",
            Self::Unbuilt => "not built yet",
        }
    }
}

/// One row of the applicability table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applicability {
    pub subsystem: Subsystem,
    pub verdict: Applies,
    /// Why — in the tool's own words, so a reader is never left to guess.
    pub because: String,
}

/// Which of Stemma's machinery applies to this body.
///
/// **ROADMAP M23's acceptance clause**, as a value. Pure, total, RNG-free: a function of
/// the profile alone, so it can be asked before a language has a single word in it.
///
/// A language with no profile gets every row as [`Applies::Yes`] — silence means an
/// ordinary speaker with an ordinary mouth, and that is what every fixture in this
/// project is.
pub fn applicability(profile: &EmbodimentProfile) -> Vec<Applicability> {
    let vocal = profile.has_vocal_tract();
    let row = |subsystem: Subsystem, verdict: Applies, because: &str| Applicability {
        subsystem,
        verdict,
        because: because.to_owned(),
    };

    if vocal {
        return Subsystem::ALL
            .iter()
            .map(|s| {
                row(
                    *s,
                    Applies::Yes,
                    "these speakers have a vocal tract, which is what every layer of \
                     Stemma is built for",
                )
            })
            .collect();
    }

    vec![
        row(
            Subsystem::Phonology,
            Applies::No,
            "a phoneme inventory is a set of things a vocal tract can do, and these \
             speakers have no vocal tract; M24 generalises a phoneme to a channel \
             signal, and until then this language has no inventory to speak of",
        ),
        row(
            Subsystem::Phonotactics,
            Applies::No,
            "a `CVC` template describes a syllable, and a signal on a non-vocal channel \
             has no syllables to shape",
        ),
        row(
            Subsystem::Prosody,
            Applies::No,
            "stress is a property of a syllable",
        ),
        row(
            Subsystem::SoundChange,
            Applies::Unbuilt,
            "the engine transforms feature bundles over segments; signal change over \
             this channel's own contrastive dimensions is M24's work, and the engine \
             itself needs no other change to do it",
        ),
        row(
            Subsystem::Morphology,
            Applies::Yes,
            "composing meaningful parts into a larger unit is about structure, not \
             about a mouth",
        ),
        row(
            Subsystem::Semantics,
            Applies::Yes,
            "meaning and its history are independent of the channel that carries them",
        ),
        row(
            Subsystem::Syntax,
            Applies::Yes,
            "§7.4's parameters are about how arguments are arranged, which every \
             channel has to solve somehow — though §18.3 expects a non-vocal channel \
             to solve it differently",
        ),
        row(
            Subsystem::Script,
            Applies::Partly,
            "a sign may map from a meaning, which works for any speaker; a sign that \
             maps from a phoneme has nothing to map from here",
        ),
    ]
}

impl Subsystem {
    /// Every subsystem, in the order the sketch lists them.
    ///
    /// A `const` array rather than a derive: the order is presentation and belongs
    /// beside the labels, and no iteration over it may reach a map (§9.4).
    pub const ALL: [Subsystem; 8] = [
        Self::Phonology,
        Self::Phonotactics,
        Self::Prosody,
        Self::SoundChange,
        Self::Morphology,
        Self::Semantics,
        Self::Syntax,
        Self::Script,
    ];
}

/// The validation checks that assume a vocal tract, and mean nothing without one.
///
/// **The applicability table made operational.** `PhonemeInventory::validate` cannot
/// know what body it belongs to — it takes no arguments — so it reports `no_nucleus` on
/// a bioluminescent species exactly as it would on a broken human language. That Error
/// is correct about a mouth and irrelevant to a creature that has none, and it is the
/// one thing standing between ROADMAP M23's fixture and validating.
///
/// So `LanguageGenome::validate` sets these aside when the profile says there is no
/// vocal tract, and says in a Note that it did and why. **Nothing else is ever
/// suppressed**: this is one short, named list with a stated reason, not a mechanism
/// for silencing inconvenient errors.
///
/// # The rule for membership
///
/// *A check that is only meaningful if the speaker produces sound at all.*
///
/// - `empty` — "the inventory has no phonemes" assumes there ought to be some. A
///   creature that speaks in light has none, and that is the honest state of its file
///   rather than a hole in it. This is the **Error** that stands between ROADMAP M23's
///   fixture and validating.
/// - `no_nucleus`, `no_consonants`, `lopsided_inventory` and the two size warnings all
///   read the consonant/vowel split, which is a fact about a mouth.
///
/// What stays: everything about a **well-formed record**. `duplicate_id`, `empty_ipa`,
/// `bad_weight` and `duplicate_ipa` are required of any language whatever its speakers
/// are made of, and a half-converted file must not be able to hide a broken row behind
/// a claim to be alien.
pub const VOCAL_TRACT_CHECKS: &[&str] = &[
    // The inventory.
    "empty",
    "no_nucleus",
    "no_consonants",
    "lopsided_inventory",
    "large_consonant_inventory",
    "large_vowel_inventory",
    // The phonotactics. Both of these end "so this language cannot generate roots
    // yet", and **`yet` is the wrong word** for a creature that will never have a
    // syllable: it reads as a to-do rather than as a fact about the body. The
    // applicability table says Phonotactics does not apply; these would quietly
    // disagree with it.
    "no_templates",
    "no_syllable_counts",
];

// -------------------------------------------------------------- validation

impl Validate for EmbodimentProfile {
    /// Structural checks, plus §18.3's grammatical consequences as **Notes**.
    ///
    /// §18.3's claims are tendencies — *"persistent scent channels encourage evidential
    /// marking"* — and this project reports tendencies without enforcing them (§17,
    /// and M17's typological harmony is the precedent this follows exactly). Nothing
    /// here is ever an Error on the grounds of being unusual, and **no message quotes a
    /// frequency**: these are consequences argued from physics, not measured across a
    /// sample of alien species that does not exist.
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();
        if self.is_empty() {
            return report;
        }

        // --- structural.
        for (i, channel) in self.channels.iter().enumerate() {
            if self.channels[..i].iter().any(|c| c.id == channel.id) {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "duplicate_channel_id",
                        format!("two channels share the id `{}`", channel.id),
                    )
                    .about(&channel.id),
                );
            }
            if channel.kind == ChannelKind::Unspecified {
                report.note(
                    "channel_without_a_medium",
                    format!(
                        "channel `{}` does not say what medium it signals in, so nothing \
                         can be said about what it implies",
                        channel.id
                    ),
                );
            }
        }

        if self.channels.is_empty() {
            report.warn(
                "no_channels",
                "this species is described but has no channel to signal on, so it has \
                 no way to say anything",
            );
        }

        // A body with no mouth and no other channel either is the one combination that
        // cannot communicate at all — worth a Warning rather than being left implied.
        if !self.has_vocal_tract() && self.non_vocal_channels().is_empty() {
            report.warn(
                "no_way_to_speak",
                "these speakers have no vocal tract and no other channel; nothing in \
                 the profile explains how they communicate",
            );
        }

        // --- §18.3's consequences. Notes, argued from the constraint, never counted.
        for channel in &self.channels {
            if channel.persistence == Persistence::Persistent {
                report.note(
                    "persistent_channel",
                    format!(
                        "`{}` persists after it is made, so a message can be found later \
                         by someone who was not there: §18.3 expects evidential marking, \
                         source tracking and territorial grammar to be worth having",
                        channel.id
                    ),
                );
            }
            if channel.simultaneity == Simultaneity::Simultaneous {
                report.note(
                    "simultaneous_channel",
                    format!(
                        "`{}` can carry several parameters at once, so morphology need \
                         not be laid out in a line: §18.3 expects simultaneous marking \
                         — mood in one parameter, tense in another",
                        channel.id
                    ),
                );
            }
            if channel.directionality == Directionality::Contact {
                report.note(
                    "contact_channel",
                    format!(
                        "`{}` requires contact or near-contact, so who is within reach \
                         is part of the situation: §18.3 expects proximity grammar and \
                         body-orientation deixis",
                        channel.id
                    ),
                );
            }
            if channel.privacy == Privacy::Broadcast
                && channel.persistence == Persistence::Persistent
            {
                report.note(
                    "public_and_lasting",
                    format!(
                        "`{}` is both public and lasting, so nothing said on it is ever \
                         said only to one hearer",
                        channel.id
                    ),
                );
            }
        }

        let limbs: u32 = self.manipulators.iter().map(|m| m.count).sum();
        if limbs > 2 {
            report.note(
                "many_manipulators",
                format!(
                    "{limbs} manipulators can be positioned independently: §18.3 expects \
                     classifier systems and agreement marked on more than one limb at once"
                ),
            );
        }

        if self.social_cognition.structure == SocialStructure::Hive {
            report.note(
                "hive_cognition",
                "the speaker is not the individual: §18.3 expects distributed pronouns, \
                 obligatory subgroup marking, and no ordinary singular",
            );
        }
        if self.social_cognition.speakers_per_utterance > 1 {
            report.note(
                "utterance_needs_several_speakers",
                format!(
                    "a complete utterance takes {} speakers, so a single speaker cannot \
                     finish a sentence alone",
                    self.social_cognition.speakers_per_utterance
                ),
            );
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A message with its `§N.N` design citations removed, so a digit scan sees only
    /// the claims.
    fn strip_citations(message: &str) -> String {
        let mut out = String::new();
        let mut chars = message.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '§' {
                out.push(c);
                continue;
            }
            // Swallow the section number that follows: digits and dots.
            while chars
                .peek()
                .is_some_and(|n| n.is_ascii_digit() || *n == '.')
            {
                chars.next();
            }
        }
        out
    }

    fn channel(id: &str, kind: ChannelKind) -> Channel {
        Channel {
            id: id.to_owned(),
            name: id.to_owned(),
            kind,
            persistence: Persistence::Unspecified,
            simultaneity: Simultaneity::Unspecified,
            directionality: Directionality::Unspecified,
            privacy: Privacy::Unspecified,
            bandwidth: Magnitude::Unspecified,
            range: Magnitude::Unspecified,
            energy_cost: Magnitude::Unspecified,
            learnability: Magnitude::Unspecified,
            noise: Magnitude::Unspecified,
            cultural_salience: Magnitude::Unspecified,
            note: String::new(),
        }
    }

    /// A bioluminescent species: no vocal tract, one light channel.
    fn glowing() -> EmbodimentProfile {
        let mut light = channel("bands", ChannelKind::Bioluminescent);
        light.persistence = Persistence::Fleeting;
        light.simultaneity = Simultaneity::Simultaneous;
        EmbodimentProfile {
            channels: vec![light],
            ..EmbodimentProfile::default()
        }
    }

    // ------------------------------------------------------------ the default

    /// **Silence is not a claim to be alien.** Every language written before M23 has an
    /// empty profile, and every one of them has speakers with mouths.
    #[test]
    fn a_language_that_says_nothing_about_its_speakers_is_treated_as_vocal() {
        let quiet = EmbodimentProfile::default();
        assert!(quiet.is_empty());
        assert!(quiet.has_vocal_tract(), "silence must not read as alien");
        assert!(
            applicability(&quiet)
                .iter()
                .all(|a| a.verdict == Applies::Yes),
            "every subsystem applies to an ordinary speaker"
        );
        assert!(
            quiet.validate().issues.is_empty(),
            "and nothing is reported"
        );
    }

    /// A profile that declares a vocal channel is vocal even without a `vocal_tract`.
    #[test]
    fn declaring_a_vocal_channel_is_enough_to_be_vocal() {
        let profile = EmbodimentProfile {
            channels: vec![channel("mouth", ChannelKind::Vocal)],
            ..EmbodimentProfile::default()
        };
        assert!(profile.has_vocal_tract());
        assert!(
            applicability(&profile)
                .iter()
                .all(|a| a.verdict == Applies::Yes)
        );
    }

    // ------------------------------------------------------- the acceptance

    /// **ROADMAP M23's acceptance.** A body with no vocal tract is told which machinery
    /// does not apply to it — subsystem by subsystem, with reasons.
    #[test]
    fn a_body_with_no_vocal_tract_is_told_what_does_not_apply() {
        let profile = glowing();
        assert!(!profile.has_vocal_tract());

        let table = applicability(&profile);
        let verdict = |s: Subsystem| {
            table
                .iter()
                .find(|a| a.subsystem == s)
                .unwrap_or_else(|| panic!("{s:?} is missing from the table"))
                .verdict
        };

        assert_eq!(verdict(Subsystem::Phonology), Applies::No);
        assert_eq!(verdict(Subsystem::Phonotactics), Applies::No);
        assert_eq!(verdict(Subsystem::Prosody), Applies::No);
        assert_eq!(verdict(Subsystem::SoundChange), Applies::Unbuilt);

        // And the parts that are about meaning rather than anatomy still work — the
        // report is a description, not a blanket refusal.
        assert_eq!(verdict(Subsystem::Semantics), Applies::Yes);
        assert_eq!(verdict(Subsystem::Syntax), Applies::Yes);
        assert_eq!(verdict(Subsystem::Script), Applies::Partly);

        // Every row carries a reason. A verdict with no `because` would leave a reader
        // guessing which is exactly what this table exists to stop.
        assert!(table.iter().all(|a| !a.because.trim().is_empty()));
        assert_eq!(table.len(), Subsystem::ALL.len(), "no subsystem is skipped");
    }

    /// A bioluminescent species with no mouth is **valid**. §17: unusual is not broken,
    /// and this is the milestone's own fixture shape.
    #[test]
    fn a_species_with_no_vocal_tract_is_valid() {
        let report = glowing().validate();
        assert!(report.is_ok(), "{report}");
    }

    // ------------------------------------------------- §18.3's consequences

    #[test]
    fn a_persistent_channel_is_told_what_it_makes_worth_having() {
        let mut profile = glowing();
        profile.channels[0].persistence = Persistence::Persistent;
        let report = profile.validate();
        let note = report
            .issues
            .iter()
            .find(|i| i.code == "persistent_channel")
            .unwrap_or_else(|| panic!("{report}"));
        assert!(note.message.contains("evidential"), "{}", note.message);
    }

    #[test]
    fn a_simultaneous_channel_expects_simultaneous_morphology() {
        assert!(
            glowing()
                .validate()
                .issues
                .iter()
                .any(|i| i.code == "simultaneous_channel")
        );
    }

    #[test]
    fn a_hive_is_told_it_has_no_ordinary_singular() {
        let mut profile = glowing();
        profile.social_cognition.structure = SocialStructure::Hive;
        profile.social_cognition.speakers_per_utterance = 3;
        let report = profile.validate();
        assert!(report.issues.iter().any(|i| i.code == "hive_cognition"));
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "utterance_needs_several_speakers")
        );
        assert!(report.is_ok(), "a hive is unusual, not broken: {report}");
    }

    #[test]
    fn many_limbs_expect_agreement_on_more_than_one_of_them() {
        let mut profile = glowing();
        profile.manipulators = vec![ManipulatorProfile {
            name: "arms".to_owned(),
            count: 8,
            dexterity: Magnitude::High,
            note: String::new(),
        }];
        assert!(
            profile
                .validate()
                .issues
                .iter()
                .any(|i| i.code == "many_manipulators")
        );
    }

    /// **No consequence message may quote a frequency** — M17's rule, and for the same
    /// reason with more force: there is no sample of alien species to have counted.
    #[test]
    fn no_consequence_message_quotes_a_frequency() {
        let mut profile = glowing();
        profile.channels[0].persistence = Persistence::Persistent;
        profile.channels[0].directionality = Directionality::Contact;
        profile.channels[0].privacy = Privacy::Broadcast;
        profile.social_cognition.structure = SocialStructure::Hive;

        for issue in &profile.validate().issues {
            // The counts this project *does* state are counts of the author's own
            // declarations — how many limbs, how many speakers — never a claimed rate.
            if issue.code == "many_manipulators" || issue.code == "utterance_needs_several_speakers"
            {
                continue;
            }
            // Section citations are stripped before the scan. `CLAUDE.md` asks for them
            // — they put the reasoning one hop away — and M17's bare digit sweep works
            // only because its messages happen to cite nothing. What the scan is for is
            // a fabricated *rate*, and `§18.3` is not one.
            let claims: String = strip_citations(&issue.message);
            assert!(
                !claims.chars().any(|c| c.is_ascii_digit()),
                "`{}` quotes a number: {}",
                issue.code,
                issue.message
            );
            for weasel in ["%", "percent", "most languages", "usually"] {
                assert!(
                    !issue.message.to_lowercase().contains(weasel),
                    "`{}` claims a statistic: {}",
                    issue.code,
                    issue.message
                );
            }
        }
    }

    /// Every enum's `Unspecified` prints as a dash and is never given the commonest
    /// value by default — M17's rule for `Headedness::Unspecified`.
    #[test]
    fn unspecified_prints_as_a_dash_and_is_never_guessed() {
        assert_eq!(Magnitude::Unspecified.label(), "—");
        assert_eq!(Persistence::Unspecified.label(), "—");
        assert_eq!(Simultaneity::Unspecified.label(), "—");
        assert_eq!(Directionality::Unspecified.label(), "—");
        assert_eq!(Privacy::Unspecified.label(), "—");
        assert_eq!(SocialStructure::Unspecified.label(), "—");
    }

    // ------------------------------------------------------------ structural

    #[test]
    fn a_duplicate_channel_id_is_an_error() {
        let profile = EmbodimentProfile {
            channels: vec![
                channel("c", ChannelKind::Chemical),
                channel("c", ChannelKind::Gesture),
            ],
            ..EmbodimentProfile::default()
        };
        assert!(
            profile
                .validate()
                .errors()
                .any(|i| i.code == "duplicate_channel_id")
        );
    }

    #[test]
    fn a_species_with_no_way_to_signal_at_all_is_told_so() {
        let profile = EmbodimentProfile {
            note: "a creature".to_owned(),
            ..EmbodimentProfile::default()
        };
        let report = profile.validate();
        assert!(
            report.warnings().any(|i| i.code == "no_channels"),
            "{report}"
        );
        assert!(
            report.warnings().any(|i| i.code == "no_way_to_speak"),
            "{report}"
        );
        assert!(report.is_ok(), "still not an Error: {report}");
    }

    #[test]
    fn a_profile_round_trips_and_refuses_a_misspelled_field() {
        let profile = glowing();
        let text = ron::ser::to_string(&profile).expect("serialise");
        assert_eq!(
            ron::from_str::<EmbodimentProfile>(&text).expect("deserialise"),
            profile
        );
        assert!(ron::from_str::<EmbodimentProfile>(r#"(chanels: [])"#).is_err());
    }

    /// An empty profile serialises to nothing, so a pre-M23 language gains no bytes.
    #[test]
    fn an_empty_profile_writes_no_fields() {
        let text = ron::ser::to_string(&EmbodimentProfile::default()).expect("serialise");
        for field in ["vocal_tract", "channels", "manipulators", "note"] {
            assert!(!text.contains(field), "`{field}` reached the file: {text}");
        }
    }
}
