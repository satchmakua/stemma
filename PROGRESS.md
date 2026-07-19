# PROGRESS — Stemma

A build log of what shipped and the notable decisions behind it. **Keep it honest**
— this is the working memory between build sessions. The forward-looking plan and
acceptance tests live in [ROADMAP.md](ROADMAP.md); this is the backward-looking
"what got done and why" companion.

**Current phase:** Phase 1 — the diachronic kernel. Next milestone: **M1 —
feature bundles & root generation**.

## State of the tree

| Crate | Holds | Status |
|---|---|---|
| `stem_core` | typed IDs, `StemmaError`, `Validate` / `ValidationReport` | working |
| `stem_phonology` | `Phoneme`, `SegmentKind`, `PhonemeInventory` + validation | M0 subset — features & phonotactics land in M1 |
| `stem_lexicon` | words, cognate sets, etymology | empty — M2 |
| `stem_soundchange` | rules, matching, traces | empty — M3 |
| `stem_genome` | `LanguageGenome` | M0 subset — grows a field per milestone |
| `stem_io` | RON/JSON load & save | working |
| `stem_cli` | the `stemma` binary | `validate`, `info`, `convert` |

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
