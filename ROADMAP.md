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

- [x] **M8 — Morphology v0.** Morphemes, simple affixation, sound change applied
  across morpheme boundaries, and the irregular paradigms that fall out of it
  (`DESIGN.md` §7.3).
  **Test:** a regular paradigm becomes irregular purely as a consequence of an
  ordered sound change, and the trace explains why. ✅ `stemma inflect
  fixtures/morphology_asterian.ron --paradigm NUMBER` gives a regular `-ka` plural;
  after `apply-rules … rules_intervocalic_voicing.ron`, `stemma paradigm` shows it
  split into `-ɡa` (after `tira`, `mena`) / `-ka` (after `tan`, `sul`), each cell's
  trace naming the rule that fired or "did not apply". 457 tests green.

- [x] **M9 — Semantics v0.** Semantic nodes instead of English gloss strings, plus
  drift events (§7.5).
  **Test:** reproduce the design's worked example — `*takala` "star" becomes
  "omen" in Coastal while staying "star" in Highland. ✅ `stemma drift out/coastal.ron
  --drift fixtures/drift_coastal.ron …` turns Coastal's reflex into "omen" through
  two recorded shifts (metaphor, then metonymy, both priestly register) while
  Highland's still means "star" — and `stemma cognates` shows all three on **one
  row**, because the join is ancestry: `star | *takala | taal "omen" | tagal`.
  `stemma trace-word … omen` prints §10.2's worked trace in full, both halves.
  504 tests green.

- [x] **M10 — Sound-change DSL.** Only now, with the semantics settled, add the
  readable rule syntax of §11.1 as a parser over the M3 structs — plus "why did
  this rule not apply?" diagnostics (§20.4).
  **Test:** the M3 rule set expressed in the DSL produces byte-identical output to
  the hand-built structs. ✅ `fixtures/rules_asterian.sc` expresses the four M3 rules
  in §11.1's syntax; `apply-rules` with the `.sc` and with the `.ron` produce files
  that `cmp` reports identical, and `the_dsl_and_the_ron_set_produce_byte_identical_output`
  pins it on the serialised bytes. 528 tests green.

- [x] **M11 — Visual explorer.** The first UI (§10), read-only to start. Not before
  the engine works — §20.5.
  **Test:** open a project, inspect a word trace, view daughters side by side. ✅
  `stemma-ui` is a native `egui` window (no WebView, no JS — one self-contained
  binary; `docs/adr/0013`). Files open by dialog, drag-and-drop, or argv; clicking a
  word shows §10.2's full history; the cognate view puts the daughters side by side.
  It holds **no logic** — every panel renders a library string through the same
  function the CLI calls, and a source-scan test bans the engine entrypoints and
  `stem_io::save` outright, which is what makes "read-only" checked rather than
  intended.

---

## Phase 3 — Breadth

_Phase 2 finished the engine; this phase makes the languages **believable**. A
103-word language is a demonstration, not a conlang — the ceiling existed only
because every word needs a meaning and the built-in list held 103 of them. M13
raised it to 673._

_The governing correction, and it overrules `DESIGN.md` §7.5's reasoning where they
conflict: **absence must be modelled, not defaulted.** §7.5 rejected Swadesh-207
partly because it would force a desert proto-language to coin a word for ice — "the
tool making a claim rather than reporting one". That argument is right about *forced*
coinage and wrong as a ceiling. A desert people have a word for ice: deserts freeze,
and you need a name for what the far-north people live on. **A language missing words
it should have is a worse failure than one carrying a word its speakers rarely use.**
So: ship breadth by default, and make the gaps deliberate (M15) rather than
accidental._

- [x] **M12 — Project concepts.** A genome may declare its own meanings alongside
  the built-in list, so a language gets the vocabulary its culture needs (nautical,
  kinship, ritual) without waiting for the compiled list to grow. Lifts the
  `--concepts` ceiling from `CONCEPT_COUNT` to "built-in + declared". The concepts
  live **inside the genome file**, so `seed`'s "reproducible from the file alone"
  contract is preserved — which is the whole reason the built-in list was compiled in
  rather than read from a sidecar (`concept.rs`), and why this must not become a
  `--concepts-file` flag.
  **Test:** a genome declaring 40 extra concepts coins 143 words, each with a stable
  id and cognate set; a pre-M12 file still coins exactly 103 and round-trips with
  zero new bytes; two runs byte-identical. ✅ `fixtures/seafarers.ron` declares 11
  (tide, keel, reef, **ice**…) and coins **114**; `--concepts 500` reports the
  language's real ceiling instead of failing an argument check;
  `declaring_project_concepts_cannot_change_a_word_already_coined` pins the prefix
  property. 533 tests green.

- [x] **M13 — A believable core vocabulary.** Grow the compiled list from 103 to
  several hundred, so a default language is usable without the author writing a
  wordlist first. **Append-only** — inserting anywhere else rewrites every word after
  it in every lexicon ever generated (`concept.rs`'s draw contract), so the existing
  103 keep their positions and every stored language is untouched. New entries carry
  `concepticon_id: None` where no mapping is verified: an unanchored concept is
  honest, a fabricated anchor is not.
  **Test:** `new-lexicon` with no flags coins the full list; the first 103 words of a
  seeded run are **byte-identical** to the pre-M13 output, proving the append changed
  nothing; homophony is reported, not prevented. ✅ **673 concepts** (103 + 570),
  organised by Buck's / the IDS's semantic fields; `new-lexicon fixtures/proto_asterian.ron`
  coins 673 words and notes 62 homophones. `--concepts 103` still hashes to M2's own
  frozen `d16ba861…`, so the append moved nothing, and
  `no_concept_added_after_the_first_hundred_and_three_claims_an_anchor` makes the
  unanchored stratum structural rather than a habit. 540 tests green.

- [x] **M14 — Derivation.** Compounds and productive affixation, so a large
  vocabulary has **etymology** instead of being N unrelated draws from the same urn.
  Reuses M8's `compose`; a derived word records what it is made of, exactly as an
  inflected cell does, and its parts are cognate-visible.
  **Test:** a language of ~300 roots yields a several-thousand-word lexicon in which
  every derived word traces to its parts; a sound change applied afterwards makes a
  compound opaque (its parts no longer recoverable by eye but still recorded), which
  is how real lexicalisation works. ✅ `fixtures/derivation_asterian.ron` declares 14
  patterns; `stemma derive` turns **673 roots into 3,978 words**, every derived one
  carrying a `BaseRef` span per part. Then `apply-rules` erodes `wikikippa`
  ("blood-debt") to **`wiiipp`** — neither part survives as a substring — and
  `stemma trace` still prints `wiki → wii` and `kippa → ipp` with both cognate sets.
  577 tests green.

- [x] **M15 — Environment & culture profile.** Model *why* a language has the
  vocabulary it has: an ecological and cultural profile that makes some concepts
  elaborated (many words), some ordinary, some borrowed-looking, some genuinely
  absent. This is the honest form of the desert/ice problem — a gap becomes a
  **stated fact about the speakers**, with a reason, rather than an accident of which
  wordlist shipped.
  **Test:** two languages over one phonology and one concept list, given different
  profiles, differ in *which* meanings are elaborated and which are missing — and
  each gap is explained in the report rather than silently empty. ✅
  `fixtures/desert_asterian.ron` and `fixtures/seafarer_asterian.ron` share a
  phonology, a concept list and a seed; both coin `star` as **`sosem`**, and differ
  only where their cultures do — `sand` 4/1, `sea` 0/4, `cattle` 4/0, `fish` 0/3.
  **`ice` inverts §7.5's own example**: the desert people have the word (they trade
  north), the islanders do not. `stemma culture` prints every gap with the trait and
  reason behind it. 611 tests green.

  *Borrowed-looking* is the one category of the four **not** implemented, and
  deliberately: a loanword is a contact fact, not an ecological one, and faking one
  without a donor language would be exactly the unsupported claim this milestone
  exists to remove. It waits for a milestone that can name the donor.

_Phase 3 is complete: the languages are broad (M13), made of themselves (M14), and
shaped by who speaks them (M15)._

---

## Phase 4 — Authoring

_Phase 3 makes languages big. This makes them **editable** without hand-writing RON._

- [ ] **M16 — Editing in the explorer.** Lift M11's read-only fence (§20.5 says
  "before adding full editing", not "never"): edit a gloss, add a word, declare a
  project concept, reorder a rule set, and save. Every edit goes through the **same
  library call the CLI would make** — the UI still holds no logic, and
  `the_ui_computes_nothing_it_could_instead_ask_a_library_for` keeps its ban on
  engine entrypoints; only `stem_io::save` leaves the list, behind an explicit save
  action. Undo is the file: nothing autosaves.
  **Test:** edit a gloss in the window, save, and `stemma validate` on the file
  agrees; a rejected edit (an id collision, a bad feature) is *reported in the
  window* and never written; a file saved from the UI is byte-identical to one the
  equivalent CLI command produces.

---

## Phase 5 — Grammar

_`DESIGN.md` §7.4. The largest remaining gap: today you cannot form a sentence._

- [ ] **M17 — Syntax profile.** §7.4's parameters as **data** — word order, head
  directionality, adposition placement, genitive and adjective order, case
  alignment, relative-clause strategy, negation, question formation, topic/focus,
  pro-drop, evidentiality, switch-reference. No engine yet: a profile, its
  validation, and a rendered grammar sketch. Typological implications are
  **reported, not enforced** (§17) — a VO language with postpositions is rare, not
  forbidden, and Stemma says which and why.
  **Test:** `stemma grammar <lang>` prints a readable sketch from stored parameters;
  a harmonically odd combination earns a Warning and still validates; two runs
  byte-identical.

- [ ] **M18 — Constructions & sentence generation.** The first sentence. A
  proposition plus the profile yields an ordered, inflected string — reusing M8's
  morphology and M14's derivation, so a generated sentence is made of words that
  already have histories. §3.3 applies unchanged: **every sentence carries a record
  of the constructions that built it**, exactly as a word carries its derivation.
  **Test:** `stemma say <lang> '<proposition>'` produces a sentence; the same
  proposition through two daughters differs *because* their profiles differ; the
  trace names every construction applied.

- [ ] **M19 — Syntactic change.** §7.4's closing claim, made real: syntax evolves.
  Case erosion forces stricter word order; topic markers become articles; serial
  verbs become auxiliaries. This is where **grammaticalization** finally lands — it
  was fenced out of M8 as "a morpheme changing role over time is diachronic
  morphology proper, a later milestone" (`docs/adr/0010`). This is that milestone.
  **Test:** a daughter whose case suffixes were eroded by an *ordered sound change*
  shows stricter word order in its profile, and the causal chain from the rule to
  the syntactic shift is on the record — not asserted by the author.

---

## Phase 6 — Script

_`DESIGN.md` §7.6. "A glyph should have ancestry just like a word" — which is the
whole design, and the reason this is a real phase rather than a font picker._

- [ ] **M20 — Glyphs & writing systems.** A script, its glyph inventory, and the
  mapping from phonology or morphology to written form (alphabet, abjad, abugida,
  syllabary, logography). A glyph is an entity with an id and a history, modelled
  the way a phoneme and a morpheme already are.
  **Test:** `stemma write <lang> <word>` renders a word in its script; the mapping
  is reported where it is lossy (an abjad drops vowels — that is the point, and the
  tool says so rather than pretending round-trip).

- [ ] **M21 — Script evolution.** §7.6's chain: pictogram → logogram → divine
  determinative → rebus sign → simplified manuscript form → modern marker. Glyph
  descent is traced exactly as word descent is, and the two are **independent** — a
  glyph may outlive the sound it once wrote, which is how real orthographies become
  historical rather than phonetic.
  **Test:** `stemma glyph-trace <lang> <glyph>` walks a glyph back to its pictogram;
  a language whose spelling froze while its pronunciation moved shows the resulting
  gap, and §17's script-history row — M7's last deferred dimension — finally scores.

---

## Phase 7 — The copilot

_`DESIGN.md` §6.5, and the phase with the sharpest constraint on it._

- [ ] **M22 — Constrained LLM assistant.** Explain a sound change; propose plausible
  daughter changes; suggest names under constraints; draft a grammar sketch from
  structured data; flag oddities.
  **The hard constraint governs absolutely, and it predates any LLM in the project:
  no LLM output may mutate a language without passing through validation** (§3.2,
  `CLAUDE.md`). Concretely: the model **proposes a `RuleSet`, a `DriftSet`, or a
  concept list** — the same authored artefacts a human writes — and the engine
  applies them by the same code path, or refuses them. There is no path from a
  model's output to a stored form that skips the engine, and prose it writes is
  labelled as prose. This is why the constraint has been enforced since M0, when
  there was nothing to enforce it against: it shapes where logic may live.
  **Test:** a proposed rule set is applied through the ordinary `apply-rules` path
  and produces a traced, reproducible result; a proposal that fails validation is
  **refused and reported**, never partially applied; with the assistant disabled
  every other command behaves identically, byte for byte.

---

## Phase 8 — Alien modality

_`DESIGN.md` §7.7 and §18. The furthest out, and the one that touches every layer:
the data model currently assumes a vocal tract throughout._

- [ ] **M23 — Embodiment profile.** §18.1's profile — auditory range, visual,
  chemical, tactile and field channels, manipulators, social cognition,
  environment — plus §18.2's channel constraints. **M15's environment work is the
  precursor**, not a detour: §18.1 names `environment: EnvironmentProfile` as a
  field *of* the embodiment profile, so the human-language ecology built there is the
  same mechanism, generalised.
  **Test:** a profile with no vocal tract but a bioluminescent channel validates,
  and the engine reports which existing machinery does *not* apply to it rather
  than silently producing vowels.

- [ ] **M24 — Non-vocal signal systems.** Generalise "phoneme" to a **channel
  signal**: a pulse, a scent, a gesture, a field modulation. Sound change becomes
  *signal* change over the channel's own contrastive dimensions. This is the
  milestone that would rename `stem_phonology`'s central abstraction, so it needs
  that abstraction to have earned the rename — everything before it must keep
  working unchanged for a vocal language.
  **Test:** a bioluminescent pulse language undergoes an ordered signal change and
  traces it, through the same engine; every existing vocal fixture produces
  byte-identical output afterwards.

---

**North star:** a user clicks a strange-looking word and gets a complete, honest
causal history of why it looks that way — and every step in that history was
produced by a rule, not a random choice.
