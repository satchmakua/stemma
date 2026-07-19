# 4. The phonological feature set is closed

- **Status:** Accepted
- **Date:** 2026-07-19

## Context

M1 replaces the placeholder `SegmentKind`-only phoneme with real distinctive
features (`DESIGN.md` §7.1). The central question is whether the set of features
is **open** — declared as data in each project file, so a language can invent
`+tone_high` or an alien `+bioluminescent` — or **closed**, compiled into the
engine as a Rust enum.

Open is the obvious answer, and it is what two independent designs proposed. The
design doc itself pushes that way: tone genesis (§7.2) creates a feature no
proto-language had, and §7.7's alien channels want a vocabulary that "place of
articulation" cannot express.

An adversarial review of those designs found the flaw. An open registry **cannot
distinguish a new feature from a misspelled one.** An author who writes `+voicee`
gets a silently registered feature that no segment values, so every rule keyed on
it matches nothing — forever, with no diagnostic. The same edit under a closed set
is a load error naming the file, the token, and the nearest real name. The
open-registry proposals also produced a family of consequential defects: an
unbounded `u8` feature id shifted past its bitset width, panicking in debug and
silently aliasing two features in release; a typo in a `parent:` reference
deleting an entire feature tier; registration order leaking into serialised
output.

## Decision

The feature set is a **closed enum of 16 binary features**, generated from a single
`features!` macro invocation together with its name and lookup tables. A
`FeatureBundle` is two `u64` bitsets giving each feature `+`, `−`, or **absent**.

Adding a feature is a source change: append a variant. Fixtures may not invent one.

The macro exists because the enum and its `ALL` table must never disagree: a
variant added to one and not the other would be dropped from `iter()`, and
`iter()` drives serialisation — so **saving a project would silently delete that
feature from every phoneme**. One input makes that unrepresentable.

## Consequences

- **The worst failure mode becomes the best one.** A typo goes from silent
  permanent misbehaviour to a load error with a "did you mean `voice`?" suggestion.
- **Nothing on disk refers to a feature by number.** Bundles serialise as signed
  *names*; discriminants are never written, and `Ord` is deliberately not derived
  on `FeatureBundle` so no export orders by bit position. Therefore **appending a
  variant, or widening the storage from `u64` to `[u64; N]`, cannot change what any
  existing file means.** This is what makes the closed set safe rather than merely
  strict.
- **Adding a feature is purely additive.** Old files do not mention it; absent is
  exactly right, because it was not a contrastive dimension in that language. No
  migration, no rewrite, no default that could be wrong. 48 bits of headroom remain
  for M3's likely additions (`tense`, `long`, `strident`, `anterior`, `aspirated`…).
- **Removing or renaming a feature is the only breaking move, and it is loud.**
- **Cost: a nine-vowel system needs a two-line PR against the enum** rather than an
  edit to a data file. That is the trade, and it is the right way round — every
  feature shipped must be assigned correctly on every segment forever, so the set
  should grow only when something needs it.
- **If §7.7 genuinely needs author-defined channels**, the extension is additive
  and quarantined: an `extra_features: BTreeMap<String, Sign>` alongside the closed
  core, behind `#[serde(default)]`. What must never happen is user-defined names
  leaking into the *core* namespace, because that is precisely what makes a typo
  indistinguishable from an intention.

### The rule to preserve

**Absent is not minus.** A feature a segment does not value means "the question
does not arise" — a plain alveolar has no rounding — never "no". Storing /f/ as
`[−strident]` rather than absent silently swells the class `[−strident]` from the
dental fricatives to every non-sibilant in the language, and nothing reports it.
Rules may reference only `+` and `−`; there is deliberately no way to ask for "the
segments where this is undefined", because that is not an attested natural class.
