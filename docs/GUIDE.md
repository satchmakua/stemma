# Stemma — a user's guide

This is the how-to. [README.md](../README.md) is the pitch and the quick tour;
[DESIGN.md](../DESIGN.md) is the reasoning; this file is what to actually type.

---

## Start here

```bash
stem
```

That builds a real 673-word language family and opens the desktop explorer on it.
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
| `derivation_asterian.ron` | 0 | Derivational affixes and patterns; `derive` turns its 673 roots into ~4,000 words. |
| `desert_asterian.ron` / `seafarer_asterian.ron` | 0 | One phonology, one seed, two ecologies — the culture-profile pair. |

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

One word per meaning, drawn from a built-in list of **673 concepts** — the Swadesh
1955 hundred, three the design's worked examples need, and a core vocabulary
organised by semantic field (kinship, weather, animals, the body, food, dwelling,
motion, quantity, time, perception, emotion, speech, society, war, law, religion, and
the pronouns and function words Swadesh leaves out). Words are drawn from the
inventory under the declared syllable templates.

The list is **append-only**, and that is a promise rather than a convention: word *n*
is drawn from the *n*-th draw of one seeded stream, so anything inserted rather than
appended would silently rewrite every word after it in every language you have ever
generated. When the list grows, your old languages come back byte-for-byte identical.

> **Why `asterian_attested` and not `proto_asterian`?** Same phonemes, but it also
> declares a **stress system**. Without prosody, a rule like "final *unstressed*
> vowel loss" can never fire — the engine tells you so (`stress_without_prosody`)
> rather than silently doing nothing, but you get a duller family. `new-lexicon`
> replaces its nine hand-authored words with 673 coined ones.

Change `--seed` for a completely different language from the same phonology. Use
`--concepts 25` for a smaller one.

### Giving your language its own vocabulary

673 is a broad vocabulary, not an exhaustive one — it has no opinion about rigging,
or your pantheon, or the four words your people have for a kind of snow nobody else
distinguishes. Declare what your culture needs in the genome itself:

```ron
concepts: [
    (key: "TIDE",   gloss: "tide", note: "twice-daily; the calendar hangs off it"),
    (key: "KEEL",   gloss: "keel"),
    (key: "REEF",   gloss: "reef"),
],
```

```bash
stemma new-lexicon fixtures/seafarers.ron --seed 7 --out out/sea.ron
# 681 words over 681 concepts
```

They append after the built-in list, so **adding one cannot change a word you already
coined**. There is no Concepticon-anchor field on a project concept, deliberately: if
a verified mapping existed, the meaning belongs on the built-in list — so a
fabricated anchor is not merely discouraged here, it is unrepresentable.

> **If the built-in list later grows into your key**, you get a Warning
> (`concepts.shadows_builtin`) and the compiled meaning wins — one key has to mean
> one thing or the comparative table cannot join on it. Delete the line; that is the
> ordinary lifecycle of a project concept, not a mistake you made. This happened to
> `fixtures/seafarers.ron` at M13: it had declared `ICE`, `SAIL` and `OAR`, and all
> three arrived on the built-in list.
>
> One caveat worth knowing: your declared concepts sit *after* the built-in block, so
> when that block grows, re-running `new-lexicon` draws **different forms for your own
> words** (the built-in ones are unaffected). Your saved file is not touched — it
> loads and validates exactly as before. Only re-coining moves them.

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

## Make words out of words

673 words is a vocabulary. It is not yet a *lexicon*, because every one of those words
is an unrelated draw from the same urn — no language is like that. Most of a real
lexicon is built from the rest of it.

```bash
stemma derivations fixtures/derivation_asterian.ron          # what patterns exist
stemma new-lexicon fixtures/derivation_asterian.ron --out out/d.ron
stemma derive out/d.ron --out out/dd.ron
```

```
Asterian (derivation) — 673 base word(s), 14 pattern(s) -> 3305 coined
3978 words over 673 concepts -> out/dd.ron
```

Two kinds of pattern, declared in the genome's `morphology`:

```ron
derivations: [
    // Productive: attaches to every word of that part of speech.
    (id: "AGENT", name: "Agent noun",
     formation: affix(affix: "m_agent", applies_to: verb),
     gloss: "one who {1}s", part_of_speech: noun),

    // Authored: you name the pairs.
    (id: "COMPOUND", name: "Noun compound",
     formation: compound(pairs: [(left: "STAR", right: "STONE")]),
     gloss: "{1}-{2}", part_of_speech: noun),
],
```

**Affixation is productive and compounding is not, deliberately.** A derivational
affix really does attach to essentially any word of its class, so applying one to
every verb reports a fact. Compounding every noun with every noun would coin 123,201
words for a 351-noun language — and whether `star` + `stone` means *meteorite* is a
fact about your culture, not a rule. So you write the pairs.

> Use `--pattern AGENT` to run one pattern, and `--limit N` to cap them all while
> experimenting. `--limit` only ever *tightens* a pattern's own cap, and it takes the
> first N bases in order rather than a random sample — so it stays reproducible.
>
> `derive` **replaces** the derived block rather than appending to it, so running it
> twice gives you the same file, not twice the lexicon.

### Then let time have it

This is the part worth doing. Evolve the derived language and the seams erode:

```bash
stemma apply-rules out/dd.ron --rules fixtures/rules_coastal.ron \
    --id late --name "Late Asterian" --years 500 --out out/late.ron
stemma trace out/late.ron w_3973
```

```
  proto      wikikippa
  │  1  r_velar_lenition   g > ɣ  [2,3) · [4,5)   → wiɣiɣippa
  │  2  r_gamma_loss       ɣ > ∅  [2,3) · [4,5)   → wiiippa
  │  3  r_apocope          a > ∅  [6,7)           → wiiipp
  modern     wiiipp

Formation:
  wiki         word     "blood"  →  wii  [w_0009 · cog_asterian_deriv_0009]
  kippa        word     "debt"   →  ipp  [w_0414 · cog_asterian_deriv_0414]

  wikikippa  →  wiiipp   — the seam has eroded; the record above is how the parts
                            are still recoverable
```

`wiiipp` contains neither `wiki` nor `kippa`. No one looking at that word could tell
it was ever a compound — which is exactly what happens to real words (`lord` was
`hlāfweard`, "loaf-guard"). Stemma still knows, because each part stored the *span* it
occupies rather than a copy of its letters, and the span is walked through the word's
own sound-change record.

That is the whole idea of the program, applied one level up from a single word.

---

## Say why your language has the words it has

A vocabulary of 673 meanings is a starting point, not a claim. Real languages
elaborate what their speakers care about and simply lack what they have never met —
and until you say which is which, every word in the dictionary is there because the
wordlist shipped it, not because these people would have it.

Declare an `environment:` in the genome:

```ron
environment: (
    summary: "The high inland waste and its two rivers; herders who trade north.",
    traits: [
        (
            id: "DESERT", name: "High desert",
            note: "No coast within a season of travel. Water is counted, not assumed.",
            elaborates: [
                (concept: "SAND", senses: [
                    "fine drifting sand, the kind that moves overnight",
                    "coarse sand that holds a footprint",
                    "sand crusted hard enough to bear a cart",
                    "sand carried on the wind, that blinds",
                ]),
            ],
            lacks: [
                (concept: "SEA",  reason: "no living speaker has seen open water"),
                (concept: "FISH", reason: "the two rivers run too fast and too cold"),
            ],
        ),
    ],
),
```

```bash
stemma culture fixtures/desert_asterian.ron
```

Every gap prints with the trait and the reason behind it, because **a gap you cannot
see is indistinguishable from an accident**:

```
  High desert  (DESERT)
    elaborates  SAND into 4 word(s):
                  · fine drifting sand, the kind that moves overnight
                  …
    lacks       SEA    — no living speaker has seen open water
    lacks       FISH   — the two rivers run too fast and too cold

  Vocabulary
    673 available meaning(s) − 16 uncoined + 10 from 5 elaboration(s) = 667 word(s)
```

Then `new-lexicon` coins that vocabulary and no other.

### Two peoples, one language

`fixtures/desert_asterian.ron` and `fixtures/seafarer_asterian.ron` carry the **same
15 phonemes, the same root shapes and the same seed**. They are one language given two
ecologies:

```bash
stemma new-lexicon fixtures/desert_asterian.ron   --out out/desert.ron
stemma new-lexicon fixtures/seafarer_asterian.ron --out out/sea.ron
```

| meaning | desert | island |
|---|---|---|
| star | `sosem` | `sosem` |
| sand | **4 words** | 1 |
| sea | **none** | **4 words** |
| fish | **none** | 3 |
| cattle | **4 words** | **none** |
| ice | 1 | **none** |

`star` is the *same word* in both, and that is deliberate: a meaning's form depends
only on its position in the concept list and the seed, so everything that differs
between these two dictionaries is the culture profile's doing and nothing else. (An
absent meaning still draws its root and throws it away, precisely so that removing one
word cannot move another.)

Look at `ice`. The desert people have a word for it — they trade north — and the
island people do not. That is the whole argument in one row: the tool is not deciding
who gets a word for ice, the author is, and the reason is in the file.

> **Notes on writing one.** The distinctions in an elaboration are *named*, not
> counted — `senses: [...]`, never `words: 4` — because four rows all glossed "sand"
> is four identical dictionary lines and no information. A `reason` is required on
> every absence. And if one trait elaborates what another says is missing, absence
> wins and you are told (`contested_concept`); you cannot have four words for a thing
> you have no word for.
>
> `stemma culture` on a language with no profile tells you what that silently asserts,
> rather than printing an empty section.

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
| `derive` `derivations` | compounds and productive affixation |
| `culture` | why this language has the vocabulary it has |
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
