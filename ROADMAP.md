# ROADMAP — Stemma

The milestone checklist — build the next unchecked milestone.

This turns [DESIGN.md](DESIGN.md) §13–§14 into a working build order. Where the
two disagree on *sequencing*, this file wins; where they disagree on *intent*,
DESIGN wins.

**Rules of the road:**
- Each milestone is an **independently runnable** slice — testable end-to-end from
  the CLI, not an internal-only refactor.
- Every milestone ends with explicit **Test** steps: what to run and what should
  happen. These are the acceptance criteria.
- Build **top-down**: a thin end-to-end slice first, then deepen.
- Check a box **only after its Test passes**, then add a `PROGRESS.md` entry.
- Counts (100 roots, 25 meanings) are budgets, not promises. Split a milestone if
  it grows.

---

## Phase 0 — Walking skeleton

- [x] **M0 — Skeleton & it runs.** Rust workspace, seven crates, typed IDs, graded
  validation, RON/JSON project I/O, and a `stemma` CLI that loads a real
  proto-language fixture and reports on it. One end-to-end test suite, green.
  **Test:** `cargo test --workspace` → green; `cargo run -p stem_cli -- validate
  fixtures/proto_asterian.ron` → prints the inventory summary and `✓ no issues`,
  exit 0; the same against `fixtures/invalid_no_vowels.ron` → three errors, exit 1.

## Phase 1 — The diachronic kernel

_This is the MVP of DESIGN §12: a deterministic diachronic lexicon engine. Nothing
outside this phase matters until it works end to end._

- [x] **M1 — Feature bundles & root generation.** 16 binary distinctive features
  with ternary storage (`+`/`−`/absent), a closed enum rather than a registry
  (`docs/adr/0004`); phonotactic templates and weighted root lengths; seeded
  generation on ChaCha20 with SHA-256 seed expansion (`docs/adr/0005`).
  `stemma generate-roots` and `stemma features`.
  **Test:** `cargo run -p stem_cli -- generate-roots fixtures/proto_asterian.ron
  --count 100 --seed 42` → 100 roots, all from the inventory, all matching a
  declared template; run twice → byte-identical; different seed → different output;
  `--count 20` is a byte-prefix of `--count 100`.

- [x] **M2 — Lexicon.** `WordEntry`, `Lexicon`, glosses, part of speech, and
  cognate-set IDs (`DESIGN.md` §8.3), plus a `concept` key the design's list omits
  — the join meaning §10.3's cognate table needs. A built-in 103-concept list
  (Swadesh 1955 + three the Phase-1 tests name). Markdown and CSV export from a new
  `stem_export` crate (`docs/adr/0006`).
  **Test:** `cargo run -p stem_cli -- new-lexicon fixtures/proto_asterian.ron --out
  out/proto.ron` → 103 entries, each with a stable `WordId` and `CognateSetId`;
  reloading yields an identical lexicon; `export-md` writes a readable dictionary
  and `export-csv` a CLDF-shaped table; both are byte-identical across runs.

- [ ] **M3 — Sound-change engine.** The heart of the program (`DESIGN.md` §7.2,
  §8.4, §11). Rules as Rust structs — **no DSL parser yet** (§20.4: start with
  structs, add a readable DSL once the semantics are settled). Segment and
  environment matching over feature bundles; ordered application; and a
  `RuleApplicationTrace` for every single application.

  Tracing is not instrumentation to add later — it is the product (§3.3). A rule
  that transforms correctly but silently is a bug.

  **Test:** intervocalic voicing turns `takala` → `tagala`; final unstressed vowel
  loss turns `tagala` → `tagal`; nasal place assimilation works; chaining the three
  in order reproduces the §10.2 worked example. Every transformed word carries a
  trace whose final form equals the stored form.

- [ ] **M4 — Forking & lineage.** `LanguageLineageGraph`, `fork_language`, and
  independent rule histories per daughter (`DESIGN.md` §8.6). Cognate-set IDs must
  survive forking — that thread is what makes the comparative view possible.
  **Test:** fork Proto-Asterian into Coastal, Highland, and Riverine; apply a
  different rule history to each; all three lexicons differ; every cognate set is
  present in all three; a trace walks unbroken from the modern form to the proto-form.

- [ ] **M5 — Cognate tables & word traces.** The two views that make the engine
  legible: the comparative table of §10.3 and the "trace this word" output of
  §10.2 — the killer feature.
  **Test:** `stemma cognates --meanings water sun star king mother` prints the §10.3
  table across all three daughters; `stemma trace-word coastal star` prints the
  full derivation, rule by rule, from proto-form to modern form.

- [ ] **M6 — The portfolio demo.** One scripted command that produces the §21
  artefact: "Growing a Language Family in 90 Seconds." Proto-language, three
  daughters, distinct histories, cognate table, five etymology traces, Markdown
  export. Snapshot-test the output (§16.4).
  **Test:** `stemma demo --out output/demo.md` runs start to finish and writes a
  document that reads as a language-family sketch. Running it twice produces
  byte-identical output.

---

## Phase 2 — Depth

_Only after Phase 1 is genuinely done. DESIGN §20.1 is explicit that scope
explosion is the top risk to this project._

- [ ] **M7 — Plausibility profile.** Grow the M0 validation report into the
  typological profile of §17: scored dimensions and specific, non-authoritarian
  warnings.
  **Test:** the §17 example warnings fire on languages that deserve them and stay
  quiet on Proto-Asterian.

- [ ] **M8 — Morphology v0.** Morphemes, simple affixation, sound change applied
  across morpheme boundaries, and the irregular paradigms that fall out of it
  (`DESIGN.md` §7.3).
  **Test:** a regular paradigm becomes irregular purely as a consequence of an
  ordered sound change, and the trace explains why.

- [ ] **M9 — Semantics v0.** Semantic nodes instead of English gloss strings, plus
  drift events (§7.5).
  **Test:** reproduce the design's worked example — `*takala` "star" becomes
  "omen" in Coastal while staying "star" in Highland.

- [ ] **M10 — Sound-change DSL.** Only now, with the semantics settled, add the
  readable rule syntax of §11.1 as a parser over the M3 structs — plus "why did
  this rule not apply?" diagnostics (§20.4).
  **Test:** the M3 rule set expressed in the DSL produces byte-identical output to
  the hand-built structs.

- [ ] **M11 — Visual explorer.** The first UI (§10), read-only to start. Not before
  the engine works — §20.5.
  **Test:** open a project, inspect a word trace, view daughters side by side.

Beyond: script evolution (§7.6), syntax and interlingua (§7.4), alien modality
(§7.7, §18), LLM copilot (§6.5). Each is a phase in its own right; none of them
should be started early.

---

**North star:** a user clicks a strange-looking word and gets a complete, honest
causal history of why it looks that way — and every step in that history was
produced by a rule, not a random choice.
