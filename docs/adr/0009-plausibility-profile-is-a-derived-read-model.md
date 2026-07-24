# 9. The plausibility profile is a derived read-model, not a second validator

- **Status:** Accepted
- **Date:** 2026-07-23

## Context

M7 grows the M0 `ValidationReport` into `DESIGN.md` §17's typological
*plausibility profile*. §17 has two halves: a block of **scored dimensions**
("Typological rarity: high", …) and a set of specific **warnings** (the 80C/2V
inventory, the extreme-change-in-little-time case, …).

`CLAUDE.md` states the hard constraint literally: *"the plausibility profile of
§17 is this same report with more checks behind it, not a separate subsystem."*
The warnings satisfy that trivially — they are `report.warn`/`report.note` calls
in the existing `Validate` impls. The **scored block** is the tension: bands like
"rarity: typical" are not naturally `Issue`s, and shipping them as a value type
plus a renderer looks, at a glance, like a second validation framework.

## Decision

**The scored block is a derived, read-only value —
`LanguageGenome::plausibility_profile() -> PlausibilityProfile` — with a pure text
renderer `render_profile`, both in `stem_genome::profile`.** It is the
`render_family` / `CognateTable` precedent: built in memory, never persisted, no
map, no float, no clock.

This is **not** a second validation subsystem, and the boundary is enforced by
four guardrails:

1. **It produces zero `Issue`s and carries no `Severity`.** An `Issue` is a
   *graded, actionable finding*; "rarity: typical" is a description true of a
   perfectly ordinary language. Firing it as a `Note` on every clean genome would
   abuse `Severity`, whose entire purpose is to separate the actionable from the
   ambient.
2. **It re-implements no check.** The bands and the warnings read **one shared set
   of threshold constants** (`LARGE_CONSONANT_COUNT`, `LARGE_VOWEL_COUNT`,
   `LOPSIDED_RATIO`, `VERY_SMALL_TOTAL`, `MARKED_CLUSTER_LEN`), so the `Rarity`
   band is `Rare` *exactly when* a size warning/note fires and `Complexity` is
   `Complex` *exactly when* a template trips `large_consonant_cluster`. A test
   pins the projection (`the_rarity_band_is_rare_exactly_when_a_size_warning_fires`).
   The band can never be a second opinion.
3. **It is derived, never stored.** No `Serialize`/`Deserialize`; not a genome
   field. Like `LineageGraph` and `CognateTable`, it is computed on demand.
4. **`stemma validate` is unchanged.** The plausibility *warnings* appear in it
   automatically, because they live in the impls it already calls — the proof
   this is "more checks on the same report." The scored block is shown by a
   separate descriptive verb, `stemma profile`, the way `genome.summary()`
   already prints beside the report.

Rejected alternatives:

- *Warnings-only, drop the scored block.* The safer literal reading of the
  constraint, but §17 shows the block *and* the warnings; the block is cheap and,
  as enums, honest. Shipping warnings-only delivers half of §17.
- *Fold bands into the report as `Note`s.* A category error (guardrail 1).

## Consequences

- **Honesty over completeness.** M7 scores only what the engine can see —
  phonology (M1) and coarse lineage (M3–M4). The five §17 dimensions that need
  unbuilt milestones (morphological irregularity → M8, semantic plausibility →
  M9, script-history → M9+, alien embodiment → §18) are rendered as explicit "not
  yet modelled → Mn" lines, never a fabricated number. Syntax/word-order is
  dropped entirely (no milestone line — §20.1 forbids inviting a syntax engine).
  §17's composite "82%" is deferred until real dimensions exist to roll up.
- **Report, do not police.** Every M7 check is a Warning or Note — an 80-consonant
  monster earns warnings and still `validate()`s. The acceptance suite pins
  `is_ok()` on the weird languages.
- **No float, no map on the profile path.** Bands are enums; the C:V ratio uses
  integer cross-multiplication (the legacy `lopsided_inventory` f32 was converted
  away, so the last float leaves the validation control path).
- **Placement.** The band enums live beside their data (`Rarity`/`rarity()` in
  `stem_phonology::inventory`; `Complexity`/`complexity()` in
  `stem_phonology::phonotactics`); `HistoricalDepth`/`PlausibilityProfile`/
  `render_profile` in `stem_genome::profile`, the only crate that sees both
  phonology and lineage and the established home for derived pure-render views.
- A future session that wants to "consolidate the two halves" of §17 into one
  mechanism should read this ADR first: they are deliberately separate because a
  description and a graded finding are different things.
