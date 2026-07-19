# PROGRESS — Stemma

A build log of what shipped and the notable decisions behind it. **Keep it honest**
— this is the working memory between build sessions. The forward-looking plan and
acceptance tests live in [ROADMAP.md](ROADMAP.md); this is the backward-looking
"what got done and why" companion.

**Current phase:** Phase 1 — the diachronic kernel. Next milestone: **M2 —
lexicon**.

## State of the tree

| Crate | Holds | Status |
|---|---|---|
| `stem_core` | typed IDs, `StemmaError`, `Validate` / `ValidationReport`, `rng` | working |
| `stem_phonology` | features, `Phoneme`, inventory, phonotactics, root generation | working |
| `stem_lexicon` | words, cognate sets, etymology | empty — M2 |
| `stem_soundchange` | rules, matching, traces | empty — M3 |
| `stem_genome` | `LanguageGenome` | grows a field per milestone |
| `stem_io` | RON/JSON load & save | working |
| `stem_cli` | the `stemma` binary | `validate`, `info`, `convert`, `generate-roots`, `features` |

---

## M1 — Feature bundles & root generation · built 2026-07-19 · ✓ verified

Phonemes now carry real distinctive features, languages declare their syllable
shapes, and `stemma generate-roots` produces reproducible root words.
**157 tests pass**; clippy is clean under `-D warnings`.

Verified by running it: `generate-roots fixtures/proto_asterian.ron --count 100`
emits roots like `kanmol`, `tatsi`, `masu`, `liwa`, `sitna`. Running it twice with
the same seed is byte-identical; a different seed differs; `--count 20` is an exact
byte-prefix of `--count 100`. `stemma features` prints each segment's resolved
matrix, and a typo in a fixture (`+voicee`) fails to load with
`19:69-19:70: unknown feature '+voicee'; did you mean 'voice'?`.

### How this was designed

M1 was specified before it was written, because the feature model is what M3's
rule engine matches against and getting its shape wrong is expensive to undo. Two
rounds of adversarial design review ran first. **The first round failed usefully:**
four proposals all tried to solve M3 inside M1 — shipping feature registries, node
geometry, ordinal scales, fixpoint resolution and hand-written parsers — and
independent judges scored every one of them 5–6/10, with one verdict reading "M1
scope is over budget against the project's own top-named risk" (§20.1). The second
round was scoped explicitly to ROADMAP M1 and scored 8/7/7.

That is the origin of the single most important decision here: **M1 is much smaller
than it first appears it should be.** No rules, no classes, no scales, no
inheritance, no DSL.

### Decisions worth knowing

**The feature set is a closed enum, not a data-declared registry.** This is
counter-intuitive — §7.2's tone genesis and §7.7's alien channels both argue for an
open namespace — and it was chosen anyway because an open registry cannot tell a
new feature from a misspelled one. `+voicee` silently becomes a real feature that
no segment carries, so every rule keyed on it matches nothing forever, with no
diagnostic. Closed, that same edit is a load error with a suggestion. Safe because
**nothing on disk refers to a feature by number** — bundles serialise as signed
names, so appending a variant cannot change what any saved file means.
[ADR-0004](docs/adr/0004-closed-feature-set.md).

**Storage is ternary; reference is binary.** A cell is `+`, `−`, or absent, where
absent means "the question does not arise" (a plain alveolar has no rounding
value). Conflating absent with minus is the single most damaging error available
here: store /f/ as `[−strident]` and the class `[−strident]` silently swells from
the dental fricatives to every non-sibilant in the language. Rules may reference
only `+` and `−`, because "the segments where this is undefined" is not an attested
natural class.

**`SegmentKind` is a phonotactic slot class, not the feature `[consonantal]`.**
This is what finally resolved the glide problem cleanly. /w/ and /j/ fill consonant
slots *and* are `[-consonantal]`; both are true, and a model that infers one from
the other has to reject an author who wrote the truth. Keeping them separate also
avoided a workspace-wide refactor of `Validate`'s signature.

**ChaCha20 + SHA-256 seed expansion, with `=` version pins.** `StdRng` is
documented as non-portable and, as of rand 0.10, may change output in a *patch*
release; `seed_from_u64` reserves the same right; and rand 0.9.0's changelog says
outright that it broke `Uniform`'s value stability — which `WeightedIndex` samples
internally. Any of the three would silently rewrite every stored language on an
unrelated `cargo update`. `default-features = false` removes the non-portable
generators from the API entirely, which is the real enforcement.
[ADR-0005](docs/adr/0005-rng-and-determinism.md).

**Weights are `u32`, not `f32`** — the one deliberate back-compat break of Phase 1.
The sampler builds a prefix-sum array in iterator order; with floats, identical
weight sets draw differently depending on summation order, and quantisation can
silently zero a small weight, leaving a phoneme in the inventory and absent from
every word. Taken now because the only affected files were fixtures this milestone
rewrote anyway. It would not be worth taking at M5.

### Evidence the model will survive M3

`crates/stem_cli/tests/reference_phonology.rs` asserts, against the real fixture,
that the design doc's own example rules are expressible today:

- `[-sonorant, -continuant, -voice]` selects exactly `{p, t, k}` (§11.1 IntervocalicVoicing)
- `[+syllabic]` selects exactly `{a, e, i, o, u}` (its `V _ V` environment)
- `[+nasal]` selects exactly `{m, n}` (nasal place assimilation)
- `[-sonorant, +dorsal]` selects `{k}` and `[+syllabic, -back]` selects `{e, i}` (§11.1 VelarPalatalization)
- /i/~/j/ and /u/~/w/ differ in exactly one feature, `syllabic`

That last one is the payoff: glide formation is a single feature flip in M3 rather
than an invention of place features out of nowhere.

### What the post-implementation review caught

M1 was reviewed by five adversarial reviewers on separate dimensions, each finding
independently reproduced by a second agent before it counted. **Three reviewers
independently found the same defect**, which is the one worth remembering:

**A vowel-only language passed `validate` with a warning and then failed to
generate.** `phonology.no_consonants` is a Warning by design — CLAUDE.md says
unusual designs are flagged, not rejected — but `RootGenerator::new` built the
consonant distribution unconditionally, so `WeightedIndex::new(vec![])` errored and
leaked a raw `rand` string. The validator and the engine disagreed about whether a
language worked, which is exactly the failure the design means to make impossible.
Fixed by preparing a slot class only if some template actually uses it.

**A one-character field typo silently changed the language.** `frequency_wieght`
was accepted, the real field took its default of 10, and on the fixture's
most-frequent vowel that rewrote **17 of 20 generated roots with no diagnostic at
any severity** — a direct hit on §9.4. `Phoneme` and `LanguageGenome` now carry
`deny_unknown_fields`. This reverses a spec decision: the spec excluded them for
forward compatibility, but the contract that actually matters ("old files keep
loading") is provided by `#[serde(default)]`, not by tolerating unknown fields.

Also fixed: generation no longer blocks on feature faults it never reads (adding
features one phoneme at a time used to break a working file halfway through);
`--count 18446744073709551615` is an argument error rather than a capacity-overflow
panic; phonotactics weight sums are overflow-checked like the inventory's;
`[+round -labial]` is flagged (a /kʷ/ authored that way would escape every
`[+labial]` rule); `Root`/`Syllable` gained `Ord`/`Hash` so M2 can dedupe without a
breaking change; the two CI determinism guards no longer **fail open** (`!` inverts
only the last command's status, so a `cargo tree` error reported success — and the
reviewer verified no frozen canary would have caught the `unbiased` feature that
guard exists for); and the nucleus property test runs 1,000 seeds rather than one.

**The golden corpus digest is unchanged across all of it** — `677f3413…` before and
after — which is the evidence that none of these fixes perturbed the draw order.

### Gotchas for the next session

- **Two bugs were caught by tests written from the spec, not by review.** (1) The
  `identical_features` check sorted its collision list but not *within* each pair,
  so reversing the inventory produced `(b, a)` instead of `(a, b)` — order-dependent
  output, the exact class of defect §9.4 forbids. (2) Nothing else; the rest
  compiled and passed first time.
- **`rand::rngs::ChaCha20Rng` exists only with `features = ["chacha"]`.** Without
  it the path is `rand_chacha::ChaCha20Rng`. Both were verified by compiling.
- **rand 0.10 renamed its traits:** `Rng` is now the core trait (`next_u64`,
  `fill_bytes`) and `RngExt` carries `.random()`. In 0.9 these were `RngCore` and
  `Rng`. Remembered code from 0.8/0.9 will not compile.
- **Three data-free canaries** (seed expansion, raw keystream, weighted-index draw
  sequence) sit alongside the corpus golden digest. No fixture edit can move the
  canaries, so a red canary means the *generator* changed while a moved digest just
  means fixture content changed. Keep that distinction — it is what stops
  re-baselining the digest from becoming a reflex.
- **`features_unspecified` is a Warning, scheduled to become an Error in M3.** It
  is a Warning now only because pre-M1 files must keep loading and M1's generator
  does not read features at all. A featureless phoneme is invisible to every rule
  M3 will write.
- **Deviation on record:** templates are concrete (`CV`, `CVC`, `V`, `VC`) rather
  than the `(C)V(C)` sugar `DESIGN.md` §15 Ticket 4 names. Parenthesis expansion
  needs an optional-slot rate nothing specifies, and its expansion order would
  silently join the RNG draw order. Rejected templates point the author at the
  explicit form. Sugar can land in M2 as a load-time rewrite.

---

## M0 — Skeleton & it runs · built 2026-07-19 · ✓ verified

Stood up the Rust workspace from the design doc: seven crates, a `stemma` CLI with
`validate` / `info` / `convert`, RON+JSON project I/O, and two reference fixtures.
**51 tests pass**; `cargo clippy --workspace --all-targets -- -D warnings` is
clean; `cargo fmt` applied.

**Verified by actually running it**, not just by the test suite: `stemma validate
fixtures/proto_asterian.ron` prints `Proto-Asterian (proto_asterian) — 15 phonemes
(10C/5V), proto, seed 42` and `✓ no issues`, exit 0. The same command against
`fixtures/invalid_no_vowels.ron` reports all three planted faults
(`phonology.bad_weight`, `phonology.duplicate_ipa`, `phonology.no_nucleus`) plus a
note, and exits 1. A RON → JSON → RON round trip through `stemma convert` returns
an identical genome.

### Decisions worth knowing

**Validation returns a graded report, not a bool.** `ValidationReport` collects
`Issue`s at Error / Warning / Note. This was chosen deliberately at M0 because
`DESIGN.md` §17 wants a *plausibility profile* — "80 consonants and 2 vowels is
possible but typologically unusual" — and a boolean validator would have forced the
engine to either reject legitimate speculative designs or say nothing useful about
them. §17 is now an extension of this report rather than a separate subsystem.
Validation also never stops at the first fault; the broken fixture proves it.

**An extra crate, `stem_genome`, that isn't in the design sketch.** §9.2 puts
`language.rs` in `stem_core`, but the genome *owns* a phonology and (later) a
lexicon and rule history, so it must depend on those crates — while they depend on
`stem_core` for IDs. That is a dependency cycle. Splitting the aggregate out keeps
`stem_core` a true foundation with no internal dependencies. Recorded in
[ADR-0002](docs/adr/0002-crate-layering.md).

**IDs are readable strings, not UUIDs.** `WordId("w_0001")`, minted from a counter,
never randomly. Traceability is the product (§3.3) and a trace full of UUIDs is
unreadable; determinism (§9.4) rules out random generation anyway. Recorded in
[ADR-0003](docs/adr/0003-readable-deterministic-ids.md).

**Two crates ship empty on purpose.** `stem_lexicon` and `stem_soundchange` have
no code, only module docs stating what belongs in them and which invariants to
preserve. Fixing the dependency edges now is cheap; rearranging them later, once
types have leaked across boundaries, is not.

### Gotchas for the next session

- **Edition 2024 match ergonomics are stricter.** `.filter(|(_, &count)| …)` over a
  `HashMap` iterator is now a hard error; it needs `.filter(|&(_, &count)| …)`.
  Cost one build cycle here.
- **RON needs `implicit_some` to read hand-authored files.** Without it, an authored
  fixture must write `romanization: Some("y")`, which defeats the point of choosing
  RON for human editing. `stem_io` enables it as a *default extension when reading*
  (so fixtures parse with or without the header) but serialises through plain
  `ron::ser::to_string_pretty` — because ron only emits the `#![enable(...)]` header
  for extensions that are *not* already defaults of the `Options` doing the
  serialising. Routing writes through the configured `Options` silently produces
  files only Stemma can read. A test now pins both halves of this.
- **`HashMap` iteration order leaks into validation output.** Duplicate detection
  sorts before reporting. Watch for this everywhere output is generated —
  determinism is a hard constraint, and this is the easiest way to break it by
  accident.
- **M1 is the load-bearing one.** The feature-bundle model has to serve M3's rule
  matching. Design it whole, against §7.1 and §11, before writing code. A fixed
  struct of `Option<bool>` fields will not survive contact with tone, phonation, or
  the alien channels of §7.7 — model it as a sparse typed bundle.
