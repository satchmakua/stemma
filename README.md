# Stemma

> A scientific language-evolution workbench: grow, fork, and trace fictional languages.

Stemma treats a constructed language as an evolving organism rather than a word
list. You define a proto-language, apply historically plausible sound changes over
simulated centuries, fork it into daughter languages, and then ask any word in any
descendant *why it looks the way it does* — and get a real answer, rule by rule,
back to the proto-form.

The name comes from textual criticism: a *stemma* is the reconstructed family tree
showing how surviving manuscripts descend from a lost original. That is exactly
this program's core data structure.

**Status:** **Phase 1 complete (M0–M6); Phase 2 underway (M7–M10 done)** — the
diachronic kernel runs end to end. Languages get a feature-based phonology,
generate seeded roots, undergo ordered sound change, fork into daughters with their
own histories, and line up in a comparative cognate table; `stemma demo` tells the
whole story as one Markdown document, and `stemma profile` scores a language
against real typological ranges without policing it. Morphology landed at M8:
`stemma paradigm` shows a regular suffix **split into irregular allomorphs by an
ordered sound change**. And meaning now has a history too — `stemma drift` makes a
word come to mean something new on one branch while its sisters keep the old sense,
**without breaking the cognate row that proves they are the same etymon**. And M10
made the rules readable: a `.sc` file writes sound changes in a plain syntax and
produces **byte-identical** output to the hand-built structs, so the notation is a
front end and can never become a second engine. 528 tests pass, and every step is
deterministic and traced. A visual explorer (M11) is next. See
[ROADMAP.md](ROADMAP.md) for the plan and [PROGRESS.md](PROGRESS.md) for what has
shipped.

---

## Run it

**Prerequisites:** Rust ≥ 1.88 with the 2024 edition (check: `cargo --version`).
Install from [rustup.rs](https://rustup.rs) if needed. No other dependencies — no
database, no Node, no system libraries.

```bash
cargo build --workspace                                    # once
cargo run -p stem_cli -- validate fixtures/proto_asterian.ron
```

You should see:

```
Proto-Asterian (proto_asterian) — 15 phonemes (10C/5V), proto, seed 42

✓ no issues
```

Try the failure path too — `fixtures/invalid_no_vowels.ron` is a deliberately
broken language, and validation reports all of its faults in one pass rather than
stopping at the first:

```bash
cargo run -p stem_cli -- validate fixtures/invalid_no_vowels.ron
```

```
! phonology.features_unspecified: this phoneme has no phonological features, so no
  sound-change rule will ever match it (`DESIGN.md` §7.1) (ph_t)
✗ phonology.bad_weight: a weight of 0 makes this phoneme unselectable; remove it
  from the inventory instead if that is what you mean (ph_k)
✗ phonology.duplicate_ipa: 2 phonemes share the IPA form /t/; they are not contrastive (t)
✗ phonology.no_nucleus: no phoneme can be a syllable nucleus, so no syllable can be formed
```

Note the grading: `✗` blocks the pipeline, `!` is a typological warning, `·` is a
note. Unusual languages are flagged, not rejected — the tool guides the creator
rather than policing them.

### Grow some words

```bash
cargo run -p stem_cli -- generate-roots fixtures/proto_asterian.ron --count 12
```

```
kanmol  tatsi  tinik  kar  nemittim  masu
milkonet  meyisa  liwa  los  leknikik  sinsa
```

Every stochastic step is seeded, so **the same file and seed always produce the
same words** — run it twice and diff the output. That is a hard constraint, not a
nicety: a language you generated last year has to still be the same language today
([ADR-0005](docs/adr/0005-rng-and-determinism.md) explains the lengths that takes).

### Look at the phonology

```bash
cargo run -p stem_cli -- features fixtures/proto_asterian.ron --ipa w
```

```
/w/  ph_w  C  w20  -syllabic -consonantal +sonorant +approximant +continuant
                   -nasal -lateral -trill +voice +labial -coronal +dorsal
                   +high -low +back +round
```

(one line per segment in the real output; wrapped here to fit)

Note `/w/` is declared `kind: consonant` yet `[-consonantal]`. Both are true: it
fills consonant slots in a syllable, but phonologically it has no radical
constriction. Keeping those two questions apart is what lets you write what is
actually the case instead of what the data model can express.

### Give it a vocabulary

```bash
cargo run -p stem_cli -- new-lexicon fixtures/proto_asterian.ron --out out/proto.ron
cargo run -p stem_cli -- export-md out/proto.ron
```

```markdown
| Form | IPA | Gloss | POS | Concept | Word | Cognate set |
| --- | --- | --- | --- | --- | --- | --- |
| aop | /aop/ | all | determiner | `ALL` | `w_0001` | `cog_proto_asterian_0001` |
| nuko | /nuko/ | ashes | noun | `ASH` | `w_0002` | `cog_proto_asterian_0002` |
| nak | /nak/ | bark | noun | `BARK` | `w_0003` | `cog_proto_asterian_0003` |
```

One word per meaning, drawn from a built-in list of 103 concepts — the Swadesh 1955
hundred, with their Concepticon anchors, plus three the design's own worked examples
need. `export-csv` writes the same data as a [CLDF](https://cldf.clld.org/)-shaped
table, including a `Segments` column of space-separated IPA that a field linguist
would normally have to reconstruct by hand.

The **cognate set** is the column that matters later. It is the thread that will
survive forking: when Proto-Asterian splits into Coastal and Highland and their
words drift apart, `cog_proto_asterian_0003` is what still says those two words are
the same word.

### Evolve it

```bash
cargo run -p stem_cli -- apply-rules fixtures/asterian_attested.ron \
    --rules fixtures/rules_asterian.ron \
    --id early_asterian --name "Early Asterian" --years 480 \
    --out out/early_asterian.ron
cargo run -p stem_cli -- trace out/early_asterian.ron w_0001
```

```
*takala  "star"  cog_asterian_attested_0001

  proto      takala        /takala/
  │ stress   TA.ka.la
  │
  0  r_0001  Intervocalic voicing
  │    k > g  [2,3)   environment  a _ a
  │    /ɡ/ is new to this language  (reference table)
  │    → tagala      /taɡala/
  │
  1  r_0002  Nasal place assimilation — did not apply
  │
  2  r_0003  Final unstressed vowel loss
  │    a > ∅  [5,6)   environment  l _ #
  │    → tagal       /taɡal/
  │
  3  r_0004  Velar lenition
  │    g > ɣ  [2,3)   environment  a _ a
  │    → taɣal       /taɣal/
  │
  modern     taɣal         /taɣal/
```

This is the design's north star working end to end: a word looks strange *for a
reason*, and the reason is on record. Note what happened at step 0 — the language
had no /ɡ/, so applying `[+voice]` to /k/ **created a phoneme**, named from a
compiled-in reference table. Rules are written over features, never letters:
"voiceless stops voice between vowels" is one rule, and whether it produces a
declared phoneme, a different declared phoneme, or a brand-new one is worked out
per site. Ordering is real, too — the fixture's `*taka` becomes `tag` under the
declared order but `tak` with the rules swapped, which is the classic
bleeding/counterbleeding contrast from historical linguistics.

### Inflect it, then let a sound change make it irregular

Morphology (M8) is where the engine pays off twice. A paradigm inflects **regularly**
— one suffix, attached to every stem. Then an ordinary sound change fires in some
cells' environments and not others, and the regular suffix splits into irregular
allomorphs, with each cell's trace explaining exactly why.

```bash
# Attach the plural suffix -ka to every stem: a regular paradigm.
cargo run -p stem_cli -- inflect fixtures/morphology_asterian.ron \
    --paradigm NUMBER --out out/number_proto.ron
# Run intervocalic voicing across the morpheme boundary.
cargo run -p stem_cli -- apply-rules out/number_proto.ron \
    --rules fixtures/rules_intervocalic_voicing.ron \
    --id early --name "Early" --years 250 --out out/number_early.ron
cargo run -p stem_cli -- paradigm out/number_early.ron --paradigm NUMBER
```

```
Paradigm — Number (NUMBER)  ·  Early

         SG     PL
  tira   tira   tiraga
  mena   mena   menaga
  tan    tan    tanka
  sul    sul    sulka

Exponents:
  ∅   zero exponent (the bare stem)
  PL   -ga / -ka   — 2 allomorphs (irregular)
    -ga   after tira, mena   ← Intervocalic voicing fired
    -ka   after tan, sul   ← the conditioning rule did not apply here
```

The suffix `/-ka/` voiced to `/-ɡa/` between vowels (after the vowel-final stems
`tira`, `mena`) but stayed `/-ka/` after a consonant (`tan`, `sul`) — the `/k/`
there is not intervocalic. Nothing declared the paradigm irregular; the irregularity
*fell out of* an ordered sound change, and `stemma trace out/number_early.ron w_0002`
walks the plural of `tira` back through the exact rule that split it.

### Let a meaning drift — without losing the family thread

Sound change moves *forms*. Semantic drift (M9) moves *meanings* — and the two are
independent. A word can come to mean something completely new on one branch while
its sisters keep the old sense, and the comparative table still shows them together,
because the table joins by **ancestry**, not by meaning.

```bash
cargo run -p stem_cli -- apply-rules fixtures/asterian_attested.ron     --rules fixtures/rules_coastal.ron     --id coastal --name "Coastal Asterian" --years 470 --out out/coastal.ron
# Two authored shifts: a metaphor, then a metonymy, both in the priestly register.
cargo run -p stem_cli -- drift out/coastal.ron     --drift fixtures/drift_coastal.ron     --id coastal_modern --name "Modern Coastal" --years 30 --out out/coastal_modern.ron
cargo run -p stem_cli -- cognates fixtures/asterian_attested.ron     out/coastal.ron out/coastal_modern.ron out/highland.ron --meanings star
```

```
meaning  asterian_attested  coastal  coastal_modern  highland
star     *takala            taal     taal "omen"     tagal
```

The form `taal` is **identical** between the two Coastal stages — only the meaning
moved. All four cells are the same cognate set, which is why they share a row.
`stemma trace-word out/coastal_modern.ron omen` then prints the whole story, both
halves:

```
*takala  "omen"  cog_asterian_attested_0001

  proto      takala        /takala/
  │ stress   TA.ka.la
  │
  0  r_ivv  Intervocalic voicing
  │    k > g  [2,3)   environment  a _ a
  ⋮
  modern     taal          /taal/

  sense      star                  sn_star
  │
  0  ev_0001  metaphor · priestly · 180y
  │    star > divine sign
  │    "§7.5's star -> divine sign, one step: the object read as intent."
  │
  1  ev_0002  metonymy · priestly · 340y
  │    divine sign > omen, royal sign
  │    "The royal house claimed the reading; §7.5's 'royal title'."
  │
  means      omen, royal sign      sn_omen, sn_royal_sign
```

Drift never touches `cognate_set`, `concept`, or the phonemic form — it writes
meaning and only meaning. That separation is what lets a dictionary say *this word
descends from `*takala` and no longer means "star"* without contradiction.

### Fork it into a family

One proto-language, three daughters, three different histories:

```bash
cargo run -p stem_cli -- fork fixtures/asterian_attested.ron \
    --rules fixtures/rules_coastal.ron  --id coastal  --name "Coastal Asterian"  --years 470 --out out/coastal.ron
cargo run -p stem_cli -- fork fixtures/asterian_attested.ron \
    --rules fixtures/rules_highland.ron --id highland --name "Highland Asterian" --years 460 --out out/highland.ron
cargo run -p stem_cli -- fork fixtures/asterian_attested.ron \
    --rules fixtures/rules_riverine.ron --id riverine --name "Riverine Asterian" --years 420 --out out/riverine.ron
cargo run -p stem_cli -- family fixtures/asterian_attested.ron out/coastal.ron out/highland.ron out/riverine.ron
```

```
Attested Asterian (asterian_attested) — proto · 15 phonemes · 9 words · 0 rules
├─ Coastal Asterian (coastal) — +470y · 17 phonemes · 9 words · 4 rules
├─ Highland Asterian (highland) — +460y · 17 phonemes · 9 words · 3 rules
└─ Riverine Asterian (riverine) — +420y · 16 phonemes · 9 words · 3 rules

cognate coverage — asterian_attested: 9 sets, 3 descendants, 9/9 present in all
```

`*takala` "star" comes out **taal** on the coast, **tagal** in the highlands, and
**tala** by the river — three languages, one ancestor, and the whole descent is on
record. The cognate ids are *copied* across every fork, never re-minted, which is
what lets `family` prove all nine sets survive into all three daughters; run
`stemma trace out/coastal.ron w_0001` to watch one word's chain walk back to
`*takala`. Descent is read from each file's `parent` field — nothing about the
family tree is stored anywhere, so it can never fall out of sync with the
languages themselves.

### Compare it by meaning

```bash
cargo run -p stem_cli -- cognates \
    fixtures/asterian_attested.ron out/coastal.ron out/highland.ron out/riverine.ron \
    --meanings water sun star king mother
cargo run -p stem_cli -- trace-word out/coastal.ron star   # == trace … w_0001
```

```
meaning  asterian_attested  coastal  highland  riverine
water    *akwa              akw      akw       akwa
sun      *sawel             sawel    sawel     sawel
star     *takala            taal     tagal     tala
king     *rekan             rean     regan     rean
mother   *mikala            mial     migal     miala
```

This is the comparative method's core table: each row is one meaning, each column a
language, and the reflexes line up because they descend from one proto-word — the
`*`-marked reconstruction in the reference column. The join is by *shared ancestry*
(cognate set), never by the meaning label, so a word whose sense later drifts still
sits in its own row. Note `king`: it resolves to `*rekan` through the word's gloss,
even though that word's underlying concept is "man" (the etymology the eventual
semantic layer will model). `stemma trace-word <file> <meaning>` is `trace` by
meaning instead of by id — the same derivation ledger, reached by what a word
*means*.

### See it all at once

```bash
cargo run -p stem_cli -- demo --out output/demo.md
```

`stemma demo` needs no arguments and no files on disk — it grows the whole
Asterian family from a compiled-in proto and three rule histories, then writes
"Growing a Language Family in 90 Seconds" as one self-contained Markdown document:
the proto glossary, each daughter's sound-change history, the comparative cognate
table, and five full etymologies from `*takala → taal` to `*mikala → miala`. It is
deterministic — two runs are byte-identical — and honest: it shows the engine's
real forms and names what is still ahead rather than faking it. This is the
Phase 1 capstone. (The exact bytes are pinned as a snapshot at
[`crates/stem_export/tests/golden/family_demo.md`](crates/stem_export/tests/golden/family_demo.md).)

### Commands

| Command | What it does |
|---|---|
| `stemma validate <file>` | Check a language for structural errors and typological oddities (exit 1 if invalid) |
| `stemma profile <file>` | Print the §17 plausibility profile: scored typological dimensions plus the report |
| `stemma info <file>` | Print a language's inventory, lineage, and root shapes |
| `stemma features <file>` | Show each phoneme's resolved feature matrix |
| `stemma generate-roots <file>` | Generate root words (`--count`, `--seed`, `--ipa`) |
| `stemma new-lexicon <file>` | Coin one word per concept (`--seed`, `--concepts`, `--out`) |
| `stemma export-md <file>` | Write the lexicon as a Markdown dictionary |
| `stemma export-csv <file>` | Write the lexicon as CLDF-shaped CSV |
| `stemma apply-rules <file>` | Apply an ordered rule set, producing a descendant language |
| `stemma inflect <file> --paradigm <id>` | Materialise a paradigm's cells into the lexicon — the regular forms (`--out`) |
| `stemma paradigm <file> --paradigm <id>` | Render a paradigm: regular on a proto, irregular after sound change, with each cell's why |
| `stemma drift <file> --drift <f>` | Apply authored semantic shifts, producing a language with new *meanings* |
| `stemma drifts <file>` | Validate and summarise a drift-set file |
| `stemma fork <parent>` | Fork a daughter (`--id`, `--name`, `--rules`, `--years`, `--out`) |
| `stemma family <files>…` | Assemble a lineage; print the family tree, cognate coverage, and report |
| `stemma cognates <files>… --meanings <m>…` | Print the comparative table of each meaning's reflexes across the family |
| `stemma trace <file> <word>` | Print a word's derivation, rule by rule |
| `stemma trace-word <file> <meaning>` | Print a word's derivation, addressed by meaning |
| `stemma demo` | Write the "Growing a Language Family in 90 Seconds" document (`--out`) |
| `stemma rules <file>` | Validate and summarise a rule-set file (`.ron`, `.json`, or `.sc`) |
| `stemma convert <in> <out>` | Convert a project between RON and JSON |

| Task | Command |
|---|---|
| Build | `cargo build --workspace` |
| Test | `cargo test --workspace` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format | `cargo fmt --all` |
| Release build | `cargo build --release` |

---

## Language files

Languages are plain RON files you can read, diff, and edit by hand — see
[`fixtures/proto_asterian.ron`](fixtures/proto_asterian.ron). There is no database
and no opaque project format; a language is a text file, and version control works
on it the way it works on code.

```ron
(
    id: "proto_asterian",
    name: "Proto-Asterian",
    seed: 42,
    phonemes: [
        (id: "ph_t", ipa: "t", kind: consonant, frequency_weight: 50,
         features: ["-syllabic", "+consonantal", "-sonorant", "-approximant",
                    "-continuant", "-nasal", "-lateral", "-trill", "-voice",
                    "-labial", "+coronal", "-dorsal"]),
    ],
    phonotactics: (
        templates: [(pattern: "CV", weight: 45), (pattern: "CVC", weight: 35)],
        syllables_per_root: [(count: 1, weight: 25), (count: 2, weight: 55)],
    ),
)
```

Features are stated literally — there is no class library and nothing is
inherited, so what you read is exactly what the engine sees. A feature left out
means **"the question does not arise"**, not "no": `/t/` above has no rounding
value because a plain alveolar simply isn't specified for rounding. That
distinction is why `[-strident]` can mean the dental fricatives rather than
accidentally meaning every non-sibilant in the language.

Misspell a feature and the file refuses to load, naming the line and guessing what
you meant:

```
19:69-19:70: unknown feature `+voicee`; did you mean `voice`?
```

---

## How to report an issue

A good report includes:

- What you ran, and what happened, in plain language.
- Any errors pasted verbatim (the single most useful thing).
- The language file involved, if it isn't one of the fixtures.

Every milestone in [ROADMAP.md](ROADMAP.md) ends with explicit **Test** steps —
those are the acceptance criteria, and a good place to start when something seems
off.

---

## Project docs

| Doc | What's in it |
|---|---|
| [DESIGN.md](DESIGN.md) | The full design and rationale — the single source of truth. |
| [ROADMAP.md](ROADMAP.md) | The milestone checklist (the plan + what's done). |
| [PROGRESS.md](PROGRESS.md) | Build log: what shipped each milestone and why. |
| [`docs/adr/`](docs/adr/) | Architecture decision records — including [why the feature set is closed](docs/adr/0004-closed-feature-set.md) and [what reproducibility actually costs](docs/adr/0005-rng-and-determinism.md). |

## Tech stack

Rust (edition 2024) workspace of eight small crates — `stem_core` (IDs, errors,
validation, seeded RNG), `stem_phonology` (features, inventory, phonotactics,
generation), `stem_lexicon` (words, concepts, cognate sets), `stem_soundchange`,
`stem_genome`, `stem_io` (persistence), `stem_export` (rendering), and the
`stemma` CLI. Serialisation via serde with RON as the authored project format and
JSON for interchange. Randomness is ChaCha20 with SHA-256 seed expansion and exact
dependency pins, because reproducibility here is measured in years. No database: a
language is a file.

A desktop/web UI is planned (M11) as a second front end onto the same crates —
deliberately not before the engine works.

## License

MIT — see [LICENSE](LICENSE).
