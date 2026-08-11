# Stemma — a user's guide

This is the how-to. [README.md](../README.md) is the pitch and the quick tour;
[DESIGN.md](../DESIGN.md) is the reasoning; this file is what to actually type.

---

## Start here

```bash
stem
```

That builds a real 103-word language family and opens the desktop explorer on it.
First run takes a minute or so (the graphics stack compiles once); after that it is
instant, because the family is cached in `out/`.

| | |
|---|---|
| `stem` | build if needed, open the explorer |
| `stem --no-ui` | build and print the comparative table, no window |
| `stem --rebuild` | throw `out/` away and regenerate from scratch |

Everything is seeded, so `stem --rebuild` gives you the **same family byte for
byte**. That is a hard guarantee, not a coincidence — see
[ADR-0005](adr/0005-rng-and-determinism.md).

### Using the explorer

- **Click a word** in the left list — that is the whole point. You get its complete
  history: every sound change that touched it, in order, with the environment that
  triggered each one, and any meaning shifts underneath.
- **Cognate table** puts the daughters side by side, joined by *ancestry* rather than
  spelling.
- **Family** shows the tree and how many cognate sets survived down each branch.
- **Profile** scores the selected language against attested typological ranges.
- Open more files with the button, or by **dropping them on the window**.

The explorer is **read-only** by design. Nothing you do in it can modify a file.

---

## A note on what ships in `fixtures/`

This trips people up, so plainly:

| file | words | what it is |
|---|---|---|
| `proto_asterian.ron` | **0** | A 15-phoneme inventory and root shapes. No vocabulary at all. |
| `asterian_attested.ron` | **9** | A *test fixture*. Each word exists to prove one engine behaviour. |
| `morphology_asterian.ron` | 0 | Four stems and a plural suffix, for the paradigm demo. |

**None of these is a language you are meant to browse.** They are the inputs. A
language with vocabulary is something you *generate*, which is what `stem` does and
what the next section explains. If you opened `asterian_attested.ron` expecting a
dictionary and found nine words, that is why.

---

## Doing it by hand

`stem` is four commands in a trench coat. Here they are.

### 1. Coin a vocabulary

```bash
stemma new-lexicon fixtures/asterian_attested.ron --seed 42 --out out/proto.ron
```

One word per meaning, drawn from a built-in list of 103 concepts (the Swadesh 1955
hundred plus three the design's worked examples need). Words are drawn from the
inventory under the declared syllable templates.

> **Why `asterian_attested` and not `proto_asterian`?** Same phonemes, but it also
> declares a **stress system**. Without prosody, a rule like "final *unstressed*
> vowel loss" can never fire — the engine tells you so (`stress_without_prosody`)
> rather than silently doing nothing, but you get a duller family. `new-lexicon`
> replaces its nine hand-authored words with 103 coined ones.

Change `--seed` for a completely different language from the same phonology. Use
`--concepts 25` for a smaller one.

### Giving your language its own vocabulary

103 is a *basic* vocabulary — the Swadesh list has no opinion about rigging, or
kinship terms, or ritual. Declare the meanings your culture needs in the genome
itself:

```ron
concepts: [
    (key: "TIDE",   gloss: "tide", note: "twice-daily; the calendar hangs off it"),
    (key: "KEEL",   gloss: "keel"),
    (key: "ICE",    gloss: "ice",  note: "they trade north"),
],
```

```bash
stemma new-lexicon fixtures/seafarers.ron --seed 7 --out out/sea.ron
# 114 words over 114 concepts
```

They append after the built-in list, so **adding one cannot change a word you already
coined**. Keys must not collide with a built-in (you'll be told). There is no
Concepticon-anchor field on a project concept, deliberately: if a verified mapping
existed, the meaning belongs on the built-in list — so a fabricated anchor is not
merely discouraged here, it is unrepresentable.

> Why in the genome and not a `--concepts-file`? Because a language must be
> reproducible **from its own file alone**. A sidecar the seed contract doesn't cover
> would mean `--seed 42` gives different languages on different machines.

### 2. Look at what you made

```bash
stemma export-md out/proto.ron          # a Markdown dictionary
stemma export-csv out/proto.ron         # CLDF-shaped, for real linguistics tools
stemma info out/proto.ron               # inventory, lineage, root shapes
stemma validate out/proto.ron           # structural errors + typological notes
stemma profile out/proto.ron            # §17's plausibility bands
```

### 3. Evolve it down a branch

```bash
stemma fork out/proto.ron --rules fixtures/rules_coastal.ron \
    --id coastal --name "Coastal Asterian" --years 470 --out out/coastal.ron
```

`fork` copies the language under a new identity and applies an ordered rule set.
Repeat with `rules_highland.ron` and `rules_riverine.ron` for sisters. Use
`apply-rules` instead when you mean "this language advanced a stage" rather than "a
sister branched off" — the file is identical either way; only the story differs.

### 4. Ask why a word looks like that

```bash
stemma trace out/coastal.ron w_0042        # by id
stemma trace-word out/coastal.ron blood    # by meaning
```

```
*wiki  "blood"  cog_asterian_attested_0009

  proto      wiki          /wiki/
  │ stress   WI.ki
  │
  0  r_ivv  Intervocalic voicing
  │    k > g  [2,3)   environment  i _ i
  │    /ɡ/ is new to this language  (reference table)
  │    → wigi        /wiɡi/
  …
```

Every line is derived, never stored prose. A rule that *could* have applied and did
not says so, with the reason.

### 5. Compare the family

```bash
stemma cognates out/proto.ron out/coastal.ron out/highland.ron out/riverine.ron \
    --meanings star water blood moon
stemma family out/proto.ron out/coastal.ron out/highland.ron out/riverine.ron
```

The table joins by **cognate set** — shared ancestry — not by meaning. That is what
lets a word drift to a new sense and keep its row.

---

## Writing your own sound changes

Rules are files. Two formats, identical meaning:

**The readable one** (`.sc`, added at M10 — see [ADR-0012](adr/0012-the-dsl-is-a-front-end.md)):

```
rules my_rules "My sound changes":

rule r_0001 "Intervocalic voicing":
  note: "Voiceless stops voice between vowels."
  at: 250
  target: [-sonorant, -continuant, -voice]
  environment: [+syllabic] _ [+syllabic]
  change: set [+voice]
```

**The structural one** (`.ron`) — see `fixtures/rules_asterian.ron`.

They produce byte-identical output; the parser is a front end, never a second engine.
Use whichever you prefer:

```bash
stemma rules my_rules.sc                                   # validate and summarise
stemma apply-rules out/proto.ron --rules my_rules.sc \
    --id stage2 --name "Stage 2" --years 300 --out out/stage2.ron
```

### The grammar

| line | meaning |
|---|---|
| `rules <id> "<name>":` | the file header, once |
| `rule <id> "<name>":` | starts a rule; **file order is chronological order** |
| `note: "…"` | prose, optional |
| `at: 250` | years since the lineage root |
| `target: [features]` | what changes. `V` = `[+syllabic]`, `C` = `[-syllabic]` |
| `environment: X _ Y` | context. `_` is the target, `#` is a word edge |
| `change: set [+voice]` | overwrite feature cells |
| `change: delete` | remove the segment |
| `change: copy place from after[0]` | take a whole feature node from a neighbour |
| `// …` | comment (`#` is taken — it means word boundary) |

**Rules name features, never letters.** `[-sonorant, -continuant, -voice]` is "the
voiceless stops", which is why it still catches a sound a *previous* rule invented.
Writing `p, t, k` would not.

### When a rule doesn't fire

Ask the tool:

```bash
stemma rules my_rules.sc      # pre-flight: can this rule ever match?
stemma trace out/stage2.ron w_0001   # per word: did it, and if not why
```

You will get one of: `target_matches_nothing` (no segment has those features),
`environment_matches_nothing`, `stress_without_prosody` (the language has no stress
system, so a stress-conditioned rule can never fire), `rule_never_applied`, or — per
site — `matched at [n,n+1) and was refused:` with the reason spelled out.

---

## Morphology

```bash
stemma inflect fixtures/morphology_asterian.ron --paradigm NUMBER --out out/n.ron
stemma apply-rules out/n.ron --rules fixtures/rules_intervocalic_voicing.ron \
    --id early --name Early --years 250 --out out/n2.ron
stemma paradigm out/n2.ron --paradigm NUMBER
```

Watch what happens: the plural suffix `-ka` is attached regularly to every stem, then
one ordinary sound change fires after vowel-final stems and not consonant-final ones —
and the regular paradigm becomes irregular by itself.

```
  PL   -ga / -ka   — 2 allomorphs (irregular)
    -ga   after tira, mena   ← Intervocalic voicing fired
    -ka   after tan, sul   ← the conditioning rule did not apply here
```

Nobody declared the irregularity. It fell out of the history.

---

## Give a word a new meaning

Meaning has its own history, separate from form:

```bash
stemma drifts fixtures/drift_coastal.ron    # inspect the drift events
stemma fork fixtures/asterian_attested.ron --rules fixtures/rules_coastal.ron \
    --id c --name Coastal --years 470 --out out/c.ron
stemma drift out/c.ron --drift fixtures/drift_coastal.ron \
    --id cm --name "Modern Coastal" --years 30 --out out/cm.ron
stemma trace-word out/cm.ron omen
```

`*takala` "star" becomes "omen" on the Coastal branch through two recorded shifts
(metaphor, then metonymy, both in a priestly register) while Highland's reflex still
means "star" — **and both keep the same cognate set**, so they stay on one row of the
comparative table. Meaning diverges; ancestry does not.

> Drift events name a **specific word id**, because semantic change is lexically
> idiosyncratic — it has no equivalent of a regular sound law. `drift_coastal.ron`
> targets `w_0001` in the *attested* fixture. Point it at a generated lexicon and it
> will correctly report that it matched nothing.

---

## The one-command showcase

```bash
stemma demo --out demo.md
```

Writes "Growing a Language Family in 90 Seconds" — a self-contained Markdown document
with the proto-language, three daughters, the comparative table, and five full
etymologies. Two runs are byte-identical.

---

## Reference

```bash
stemma --help          # every command
stemma <cmd> --help    # its options
```

| command | what it does |
|---|---|
| `validate` `profile` `info` `features` | inspect a language |
| `generate-roots` `new-lexicon` | make words |
| `apply-rules` `fork` | evolve and branch |
| `trace` `trace-word` | why a word looks like that |
| `cognates` `family` | compare a family |
| `inflect` `paradigm` | morphology |
| `drift` `drifts` | meaning change |
| `rules` | validate a rule file (`.ron`, `.json`, or `.sc`) |
| `export-md` `export-csv` `demo` | get data out |
| `convert` | RON ↔ JSON |
| `stemma-ui [files…]` | the desktop explorer |

### File formats

Everything is a plain text file you can read, diff, and put in git. `.ron` is the
native format, `.json` works everywhere, `.sc` is the rule DSL. There is no database
and no project binary — a language *is* its file, and the seed lives inside it so the
result is reproducible from that file alone.

---

## What Stemma does not do *yet*

Named honestly, each with the milestone that fills it. None of these is a permanent
exclusion — see [ROADMAP.md](../ROADMAP.md).

| gap | today | planned |
|---|---|---|
| **Syntax** — you cannot form a sentence. No grammar engine, no parser, no translation. | a *lexicon* engine with morphology | **M17** syntax profile → **M18** sentence generation → **M19** syntactic change |
| **Writing systems** — script evolution (§7.6) is designed but unbuilt. | forms render in IPA and romanisation only | **M20** glyphs → **M21** script evolution |
| **Alien modality** — §7.7's pulse/scent/gesture languages. | the data model assumes a vocal tract throughout | **M23** embodiment profile → **M24** non-vocal signal systems |
| **An LLM assistant** (§6.5). | nothing, by design | **M22** — and the constraint holds: a model *proposes* a rule set, the engine applies or refuses it. No path to a stored form skips the engine. |
| **Editing in the UI** — read-only (§20.5). | inspect only; nothing autosaves | **M16** |

Two of these are worth understanding rather than just waiting for.

**The LLM one is a constraint, not an absence.** "No LLM output may mutate a language
without passing through validation" has been enforced since M0, when there was
nothing to enforce it against, because it decides *where logic may live*. When M22
lands, the assistant will write the same authored artefacts you would — a `.sc` rule
file, a drift set — and they go through `apply-rules` like anything else, or they are
refused. That is the only door.

**Alien modality is last because it is hardest, not because it is least wanted.**
Generalising "phoneme" to "channel signal" touches every layer, and the test for
M24 is that every existing vocal language still produces byte-identical output
afterwards. The abstraction has to earn the rename.
