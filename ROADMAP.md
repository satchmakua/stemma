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

- [x] **M3 — Sound-change engine.** Rules as structs over feature bundles —
  target, adjacency-window environment, `set`/`copy`/`delete` changes; simultaneous
  application over a frozen snapshot; a three-tier bundle-to-symbol resolver that
  can **mint a phoneme the language did not have** (voicing /k/ creates /ɡ/,
  U+0261, from a compiled-in 20-row reference table); a minimal prosody store
  (fixed-position stress) so "final *unstressed* vowel loss" is honest; and a
  stored per-word `Derivation` that replays to the stored form. `stemma
  apply-rules`, `stemma trace`, `stemma rules`. **No RNG anywhere in the engine.**
  **Test:** `stemma apply-rules fixtures/asterian_attested.ron --rules
  fixtures/rules_asterian.ron …` reproduces §10.2's chain `takala → tagala →
  tagal → taɣal` exactly; nasal place assimilation resolves three ways from one
  rule (declared /m/, declared /n/ via absence-copying, minted /ŋ/); `*taka`
  proves rule order is observable (`tag` vs `tak`); every word's trace replays to
  its stored form; two runs are byte-identical.

- [x] **M4 — Forking & lineage.** `LanguageGenome::fork` (a verbatim copy under a
  new identity, with a parent edge back — cognate sets copied, never minted), the
  in-memory `LineageGraph` whose edges are *derived* from `parent` fields and never
  stored, family-level validation, and `stemma fork` / `stemma family`
  (`docs/adr/0008`). No `LineageEdgeKind` and no `WordEntry.ancestor` until a
  producer needs them; the graph uses no map anywhere.
  **Test:** `stemma fork fixtures/asterian_attested.ron --rules fixtures/rules_coastal.ron …`
  (and Highland, Riverine) yields three sisters whose lexicons differ pairwise —
  `takala` → **taal** / **tagal** / **tala**; `stemma family …` reports **9/9**
  cognate sets present in all three; every daughter's trace replays to its stored
  form and `stemma trace out/coastal.ron w_0001` walks unbroken back to `*takala`;
  two fork runs are byte-identical.

- [x] **M5 — Cognate tables & word traces.** The two meaning-addressed views:
  §10.3's comparative table (`stemma cognates`), joined by cognate set so meaning
  drift can never drop a reflex, and §10.2's derivation addressed by meaning
  (`stemma trace-word`, reusing the M3 renderer). One shared `Lexicon::by_meaning`
  resolver matches a word's displayed gloss, so `king` finds the word glossed
  "king". MOTHER `*mikala` joined the reference fixture (now nine words).
  **Test:** `stemma cognates fixtures/asterian_attested.ron out/coastal.ron
  out/highland.ron out/riverine.ron --meanings water sun star king mother` prints
  the §10.3 table (`star` → `*takala`/taal/tagal/tala; `king` → `*rekan`/rean/…;
  `mother` → `*mikala`/mial/migal/miala); `stemma trace-word out/coastal.ron star`
  prints the full derivation and is byte-identical to `trace … w_0001`.

- [x] **M6 — The portfolio demo.** `stemma demo` — one command, no arguments —
  emits the §21 artefact "Growing a Language Family in 90 Seconds" as a
  self-contained Markdown document: the proto-language, its three daughters with
  their rule histories, the comparative cognate table, and five full etymology
  traces. Pure composition — a `stem_genome::grow_family` helper builds the family,
  a new `stem_export::write_cognate_table_markdown` projects the table, and a pure
  `write_family_demo` stitches it all — plus honest scope: it shows the engine's
  real `tagal` (never §21's unreachable `tazal`) and names meaning drift as
  forthcoming rather than faking it. **No engine work.**
  **Test:** `stemma demo --out output/demo.md` runs start to finish and writes a
  language-family sketch (star row `*takala | taal | tagal | tala`, five fenced
  derivations); two runs are byte-identical; a `stem_export` golden pins the exact
  bytes and engine-independent canaries pin the renderer.

---

## Phase 2 — Depth

_Only after Phase 1 is genuinely done. DESIGN §20.1 is explicit that scope
explosion is the top risk to this project._

- [x] **M7 — Plausibility profile.** §17's typological profile grown into the
  *same* `ValidationReport`: three new phonological/lineage Warnings/Notes
  (`large_consonant_inventory`, `large_vowel_inventory`, `large_consonant_cluster`,
  `high_change_density`), plus a derived scored-dimensions block
  (`plausibility_profile()` + `render_profile`, bands not percentages) shown by
  `stemma profile`. The bands read the *same* threshold constants the warnings do,
  so the score can never disagree with the check (`docs/adr/0009`); dimensions with
  no data (morphology M8, semantics M9, script M9+, alien §18) render "not yet
  modelled", never a fabricated number. Report, do not police — every new check is
  a Warning/Note, never an Error.
  **Test:** the §17 example warnings fire on languages that deserve them (an
  80-consonant inventory, a `CCCVCC` template — each still `is_ok()`) and stay
  quiet on Proto-Asterian and all three of its daughters.

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
