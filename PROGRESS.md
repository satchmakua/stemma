# PROGRESS — Stemma

A build log of what shipped and the notable decisions behind it. **Keep it honest**
— this is the working memory between build sessions. The forward-looking plan and
acceptance tests live in [ROADMAP.md](ROADMAP.md); this is the backward-looking
"what got done and why" companion.

**Current phase:** Phase 1 — the diachronic kernel. Next milestone: **M3 —
sound-change engine**.

## State of the tree

| Crate | Holds | Status |
|---|---|---|
| `stem_core` | typed IDs, `StemmaError`, `Validate` / `ValidationReport`, `rng`, `suggest` | working |
| `stem_phonology` | features, `Phoneme`, inventory, phonotactics, root generation | working |
| `stem_lexicon` | `WordEntry`, `Lexicon`, the 103-concept list, cognate-set minting | working |
| `stem_soundchange` | rules, matching, traces | empty — M3 |
| `stem_genome` | `LanguageGenome` | grows a field per milestone |
| `stem_io` | RON/JSON load & save | working — **untouched since M0** |
| `stem_export` | Markdown dictionaries, CLDF-shaped CSV | working |
| `stem_cli` | the `stemma` binary | `validate`, `info`, `convert`, `generate-roots`, `features`, `new-lexicon`, `export-md`, `export-csv` |

---

## M2 — Lexicon · built 2026-07-19 · ✓ verified

Languages now have words. `stemma new-lexicon` coins one root per concept from a
built-in 103-item list, `export-md` writes a dictionary and `export-csv` a
CLDF-shaped table. **232 tests pass**; clippy clean under `-D warnings`.

Verified by running it: `new-lexicon fixtures/proto_asterian.ron --out
out/proto.ron` produces 103 entries — `aop` "all", `nuko` "ashes", `nak` "bark",
`sa` "big" — each with a stable `WordId` and `CognateSetId`; reloading yields an
identical lexicon; the dictionary and the CSV are byte-identical across runs. The
homophone check found 5 real collisions (`a`, `ni`, `nim`, `wa`) and reported them
as a Note without making the language invalid.

### Decisions worth knowing

**Export is a new crate, not a relaxation of `stem_io`.** `DESIGN.md` §9.2 puts
`markdown.rs` inside `stem_io`, but `stem_io`'s own module docs say it is generic
over serde and must not know the domain — and a Markdown dictionary cannot be
written that way, since a word's rendered form needs the phoneme inventory. The
distinction the split encodes: **persistence is total, reversible and domain-blind;
rendering is lossy, opinionated and domain-specific.** `stem_io/src/` was not
touched by this milestone, not one line. [ADR-0006](docs/adr/0006-export-is-a-separate-crate.md).

**A `concept` field that §8.3 does not list.** §8.3 offers `glosses` (free strings
that drift) and `semantic_nodes` (M9, and `SemanticNodeId` does not exist yet).
Neither can be the cross-language join key §10.3's cognate table needs, so
`ConceptKey` was added. It is deliberately **not** the cognate set: a concept is
shared *meaning*, a cognate set is shared *ancestry*, and Latin *caput* "head" →
French *chef* "chief" have one and not the other. At M2 they are in exact bijection
and a future reader will want to delete one — a test exists to stop that.
[ADR-0007](docs/adr/0007-word-identity-and-cognate-sets.md).

**Swadesh-100 does not contain `king` or `mother`** — and ROADMAP M5's own
acceptance command is `stemma cognates --meanings water sun star king mother`. The
design panel caught this. Three concepts (`MOTHER`, `KING`, `STORM`) are appended
after the hundred, under an explicit rule: *a meaning named by a Phase-1 acceptance
test or a DESIGN worked example must be representable*. Appending rather than
interleaving is what preserves the draw contract's prefix property. Without them,
M5 could not run its own test, and reopening the concept schema at M5 is exactly
the deferred cost this milestone exists to avoid.

**Concepticon anchors were fetched, not remembered.** Each Swadesh concept carries
its Concepticon id. Two values that circulate in secondary sources are wrong and
the fetched ones are used: `hair` is 1036 (not 1040) and `root` is 668 (not 670).
`KING` and `STORM` could not be verified, so their `concepticon_id` is `None` — a
plausible-looking integer under a column bearing an external authority's name would
be a false provenance claim in a program whose premise is provenance.

**`phonemic_form` is a `Root`, never a `String`.** A rendered form stored beside
the segments is a second source of truth that desynchronises the first time M3
mutates a segment, undetectably. `written()` and `ipa()` are views.

**The lexicon draws on its own RNG stream.** `RngDomain::Lexicon`, so the Nth word
of `new-lexicon --seed 42` is deliberately *not* the Nth root of `generate-roots
--seed 42`. Sharing would freeze the lexicon builder's draw budget to the root
generator's forever. `generate-roots` is a scratchpad; `new-lexicon` is an artifact.

### Gotchas for the next session

- **`build_proto_lexicon` must not call `inventory.validate()`.** `RootGenerator::new`
  deliberately filters feature-only codes out of its gate so a half-featured file
  still generates; re-validating would make `new-lexicon` refuse a language
  `generate-roots` accepts — M1's validator/engine defect reopened one axis over.
  `a_language_that_can_generate_roots_can_seed_a_lexicon` guards it.
- **Edit distance is Damerau-Levenshtein, not plain Levenshtein, and that was a
  bug fix.** Under plain Levenshtein a transposition costs 2, so `NOES` tied with
  `NOSE`, `NEW` and `NOT` — and the deterministic tie-break returned `NEW`. Since
  transposition is the most common typing error, counting it as one edit makes the
  right word win. The suggester moved to `stem_core::suggest` so features and
  concepts share one implementation.
- **`--out` rewrites the stored seed.** `--seed 7 --out f.ron` must not write a file
  saying `seed: 42` while holding words drawn from stream 7; the genome's seed
  promises reproducibility *from the file alone*, and this is the first command
  that persists a stochastic result.
- **Two golden tiers again.** The data-free canary in `stem_export` (a hand-built
  4-phoneme genome) cannot be moved by any fixture edit; the fixture goldens and
  the `d16ba861…` lexicon digest legitimately move when fixture content changes.
  Keep them distinguishable.
- **The reference fixture is unchanged and stays lexicon-less.** It is a proto
  *definition*, the thing `new-lexicon` is run against. `skip_serializing_if` keeps
  `convert` from adding an empty `lexicon: []`, so its round trip is still
  byte-identical — a test asserts it.
- **M4's whole cognate obligation is one line:** copy `cognate_set` verbatim, never
  mint. `scoped_cognate_set` is the only minting site in the workspace and a
  source-scanning test enforces it.

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
