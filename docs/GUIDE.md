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
- **Edit** changes the language: relabel a word, add one, declare a concept. Nothing
  is written until you press **Save**, and unsaved changes are marked `● unsaved`.

Every edit in the window is the *same library call* the matching `stemma` command
makes, so a file saved here and a file saved from the terminal are byte-identical.
**Undo is the file** — nothing autosaves, so close without saving to discard.

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
| `grammar_asterian.ron` / `grammar_svo_asterian.ron` | 0 | One phonology, one seed, two grammars — the sentence-generation pair. |
| `grammar_free_asterian.ron` | 0 | Free order with a live case system — the "before" of a grammaticalization. |

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

## Say how your language builds a clause

```bash
stemma grammar fixtures/grammar_asterian.ron
```

```
Grammar — Asterian (grammar)

  Word order        SOV                           object before verb
  Headedness        head-final                    derived from the orders below, never stored
  Adpositions       postpositions                 *the house in*
  Genitive          genitive-noun                 *the king's road*
  Adjective         noun-adjective                *the stone black*
  Alignment         ergative-absolutive           the one who walks patterns with the one who is hit
  Relative clause   prenominal                    before the noun it modifies
  Negation          affix on the verb
  Questions         question particle
  Pro-drop          subject pronouns droppable
  Evidentiality     two-way
  Switch-reference  marked on the dependent verb

  Typologically harmonic: every stated order agrees with the others.
```

Declare it in the genome:

```ron
syntax: (
    note: "Rigidly verb-final, with a rich case system.",
    word_order: sov,
    adpositions: postpositions,
    genitive: genitive_noun,
    adjective: noun_adjective,
    alignment: ergative_absolutive,
    relative_clause: prenominal,
    negation: affix,
    question: particle,
    pro_drop: yes,
    evidentiality: two_way,
    switch_reference: marked,
),
```

Leave out anything you have not decided — it prints as `—` rather than quietly
becoming whatever is commonest. A language with no `syntax:` at all is told it has
none; it is **not** given SVO by default.

> **There is no `head_directionality` field**, deliberately. It is a summary of the
> orders above it, so storing it would be a second source of truth that disagrees with
> the first the moment you edit one line. `stemma grammar` works it out.
>
> Adjective order is left out of that calculation on purpose: it is the one noun-phrase
> parameter that famously does not track the others, so counting it would call half the
> world's languages mixed for a reason that is not about headedness.

### It tells you what is rare. It does not stop you.

Change `postpositions` to `prepositions` and run it again:

```
  Notes on harmony:
    · object-verb order usually goes with postpositions, not prepositions;
      this combination is attested but uncommon
    · this language is head-initial in some constructions and head-final in others;
      that is ordinary — English and most of Europe are mixed

  These describe what is common, not what is correct. A rare language is
  a design; Stemma reports it and does not refuse it (§17).
```

`stemma validate` agrees: one warning, nothing blocking. **No combination of these
parameters is ever an error** — there is a test that sweeps 960 of them to make sure.

The tendencies are Greenberg's word-order universals and the cross-linguistic work
that refined them. Notice what the messages do *not* say: no percentages. It would be
easy to write "only 4% of languages do this" and impossible for this program to back
it up, so it doesn't — the same rule that keeps invented Concepticon ids out of the
concept list.

> **This describes; it does not generate.** You cannot yet ask Stemma to *say*
> anything in your language. That is M18, and this profile is what it will read.

---

## Say something

```bash
stemma new-lexicon fixtures/grammar_asterian.ron --out out/sov.ron
stemma say out/sov.ron 'SEE(KING, STAR)'
```

```
mostair sosema ponti
  SEE(KING, STAR)  ·  Asterian (grammar)

  mostair  agent      king-ERG  [w_0102 · cog_asterian_grammar_0102]
  sosema   patient    star-ABS  [w_0080 · cog_asterian_grammar_0080]
  ponti    predicate  see       [w_0072 · cog_asterian_grammar_0072]

Constructions:
  0  case_marking  marked the agent `ERG`
     because alignment = ergative-absolutive
  1  case_marking  marked the patient `ABS`
     because alignment = ergative-absolutive
  2  clause  ordered the clause SOV
     because word_order = SOV
```

Every word is a real dictionary entry with a real id, so you can go straight from a
sentence to any word's full history: `stemma trace out/sov.ron w_0102`.

### The notation

You give it a **proposition** — meanings, not words:

```
PREDICATE(ARGUMENT, ARGUMENT)     a transitive clause
PREDICATE(ARGUMENT)               an intransitive one

ARGUMENT := CONCEPT [":" ADJECTIVE] ["/" POSSESSOR]
```

```bash
stemma say out/sov.ron 'SEE(KING:BIG/PRIEST, STAR)'   # the priest's big king sees the star
```

Concept keys, exactly as they appear in the dictionary — so the notation needs no
vocabulary of its own and cannot drift from one. `:` and `/` are shell-safe unquoted.

### One proposition, two grammars

`grammar_asterian.ron` and `grammar_svo_asterian.ron` have the **same phonemes and the
same seed**, so they coin the same words. Everything that differs is the grammar:

```bash
stemma new-lexicon fixtures/grammar_svo_asterian.ron --out out/svo.ron
stemma say out/sov.ron 'SEE(KING:BIG, STAR/PRIEST)'
stemma say out/svo.ron 'SEE(KING:BIG, STAR/PRIEST)'
```

```
head-final:    mostair sa taot sosema ponti
head-initial:  sa mostau ponti sosemta taot
```

Same five words. Different order, different side for the adjective and the possessor,
different case endings.

### Watch the alignment

That last difference is the one that is not a reordering at all:

```bash
stemma say out/sov.ron 'SEE(KING, STAR)'   # transitive
stemma say out/sov.ron 'SEE(KING)'         # intransitive
```

```
                 transitive     intransitive
  ergative:      mostair        mostaa        ← two different endings
  nominative:    mostau         mostau        ← one
```

An **ergative** language marks the agent of a transitive clause and leaves the lone
argument of an intransitive one alone with the object's ending. A **nominative** one
marks both the same. That is the whole content of those two words, and your language
does it because you wrote `alignment:` in its genome.

> **What it will not do.** One clause: a verb, up to two arguments, one adjective and
> one possessor each. No subordination, no coordination, no tense, no agreement, and no
> relative clauses — even though the profile records a relative-clause strategy.
>
> If your language has no morpheme for a case it needs, you still get the sentence,
> unmarked, with a line telling you which morpheme to declare. If it has no *word* for
> a concept, that is an error: there is no sentence to be had, and Stemma will not coin
> one on the spot.

---

## Let the grammar change

Everything so far has evolved *forms* and *meanings*. Grammar evolves too, and the
interesting part is why.

`grammar_free_asterian.ron` is a language with **free constituent order and a working
ergative case system**. Those two go together: if every noun says what it is doing by
its ending, order is free to carry emphasis instead.

```bash
stemma new-lexicon fixtures/grammar_free_asterian.ron --out out/old.ron
stemma apply-rules out/old.ron --rules fixtures/rules_case_erosion.sc \
    --id middle --name "Middle Asterian" --years 600 --out out/middle.ron
stemma shift out/middle.ron --changes fixtures/shift_asterian.ron \
    --id modern --name "Modern Asterian" --years 60 --out out/modern.ron
```

```
Syntactic history — Modern Asterian

  0  Word order fixes as the ergative erodes  (sx_0001)
     word order became SOV
     because `m_erg` surfaces on no word of this language; `r_e02` is the recorded
     sound change that erased it
     the cause is on the record: sound change `r_e02`
```

The rule set is two perfectly ordinary sound changes — final rhotic loss, then final
unstressed vowel loss. Neither knows what a case is. Between them they eat both
suffixes, and the language changes shape:

```
                word order   alignment              SEE(KING, STAR)
  Old Asterian  free         ergative-absolutive    mostair ponti sosema
  Modern        SOV          neutral                most sosem pont
```

### What you write, and what Stemma works out

A change file has a **condition** and a **consequence**:

```ron
(
    id: "sx_0001",
    name: "Word order fixes as the ergative erodes",
    when: case_marker_lost(morpheme: "m_erg"),
    then: word_order(sov),
),
```

The `when:` is **checked, not believed**. `stemma shift` inflects every noun in the
lexicon with that morpheme, runs the language's own recorded sound changes over the
results, and looks at what is left. Run it on the *un-eroded* language and it refuses:

```
· sx_0001 did not apply: `m_erg` still surfaces as `-ir` on at least one word
  ("ashes"), so it has not been lost
```

The `then:` is yours, and that is deliberate. A language that loses its case marking
might fix its word order, or lean harder on adpositions, or grow verb agreement.
Which one *your* people did is not something a program can work out, and Stemma will
not pretend otherwise.

What neither of you writes is **the name of the sound change**. That is found by
replaying each word's own derivation to see which step emptied the suffix — so the
line "the cause is on the record: sound change `r_e02`" is a finding, not an echo of
your file.

> **Order is what does it.** `-ir` ends in a rhotic, so vowel loss alone cannot reach
> the vowel behind it — the rhotic has to go first. Swap the two rules in
> `rules_case_erosion.sc` and the ergative survives, so the shift is refused. That is
> the same "rule order is observable" M3 proved on a single word, now deciding whether
> a language keeps its case system.
>
> **Only case erosion is modelled.** Topic markers becoming articles and serial verbs
> becoming auxiliaries need an article, an auxiliary and a serial verb to exist in the
> model first, and none of them does.

---

## Write it down

A language has sounds and words; a *written* language also has signs, and a mapping
between the two. That mapping is allowed to lose things — and that is the whole reason
this is a milestone rather than a font picker.

`fixtures/written_asterian.ron` is the reference Asterian inventory under **three**
scripts. Same fifteen sounds, three ways of setting them down:

```bash
stemma scripts fixtures/written_asterian.ron
```

```
Writing — Written Asterian

  The Kirran alphabet  (kirran, alphabet)
    The clerks' script: one sign per sound, vowels included. Late, and invented for bookkeeping.
    15 sign(s)
    writes every sound in the language

  The Tirran abjad  (tirran, abjad)
    The old temple hand. Consonants only; a reader who knows the language supplies the
    vowels, and one who does not is not the intended reader.
    10 sign(s)
    does not write: /a/ /e/ /i/ /o/ /u/

  The Emmen signs  (emmen, logography)
    Older than either. One sign, one meaning, no sounds…
    5 sign(s)
    writes 5 meaning(s), and no sounds at all — a word with no sign of its own cannot be written in it
```

Coin the vocabulary, then write one word in each:

```bash
stemma new-lexicon fixtures/written_asterian.ron --out out/written.ron
stemma write out/written.ron star
```

```
sosem
  sosem  "star"  ·  The Kirran alphabet (alphabet)

  s     /s/           [g_s]
  o     /o/           [g_o]
  s     /s/           [g_s]
  e     /e/           [g_e]
  m     /m/           [g_m]

  every sound is written; this spelling can be read back exactly.
```

Now the same word in the abjad:

```bash
stemma write out/written.ron star --script tirran
```

```
SSM
  sosem  "star"  ·  The Tirran abjad (abjad)

  S     /s/           [gt_s]
  S     /s/           [gt_s]
  M     /m/           [gt_m]

  Not written:
    /o/  — by design
    /e/  — by design

  2 sound(s) are not written, which is what an abjad does — a reader supplies them
  from knowing the language. This spelling does not round-trip, and is not meant to.
```

`SSM` is not a broken spelling of *sosem*. It is what an abjad does — Semitic scribes
wrote three consonants for a word with three vowels in it for three thousand years,
and readers supplied the rest from knowing the language.

### The last line is the point

There are two easy ways for a tool to lie here, and Stemma does neither:

1. **Invent the missing signs**, so the round trip works. A comfortable abjad with
   vowel letters is an alphabet, and you asked for an abjad.
2. **Drop them quietly**, so `SSM` looks complete and you believe you could read it
   back.

So the loss is listed sound by sound and stated in a sentence — *including when there
is none*. A tool that only ever spoke up about failure would let silence read as
completeness, which is the same bug wearing a different hat.

The claim is also checked rather than printed: replay what each sign carried and see
whether you get the word back. The alphabet's spelling reconstructs `sosem` exactly;
the abjad's does not, and a test asserts both.

### A sign for the meaning

The third script writes no sounds at all:

```bash
stemma write out/written.ron star --script emmen
```

```
★
  sosem  "star"  ·  The Emmen signs (logography)

  ★     = STAR        [ge_star]

  Not written:
    /s/  — by design
    …

  5 sound(s) are not written, which is what a logography does…
```

A logography can only write the words that earned a sign. Ask it for one it has not
got, and it says so — and says something *different*, because a word written
incompletely and a word not written at all are not the same fact:

```bash
stemma write out/written.ron king --script emmen
```

```
  mosta  "king"  ·  The Emmen signs (logography)

  (no sign in this script wrote any part of this word)
  …
  `emmen` has no sign for this word, so none of its 5 sound(s) reached the page at
  all. Nothing was written — which is not the same as something written incompletely.
```

### Declaring a script

```
scripts: [
    (
        id: "tirran",
        name: "The Tirran abjad",
        kind: abjad,            // alphabet · abjad · abugida · syllabary · logography
        glyphs: [
            (id: "gt_s", form: "S", name: "the tooth"),
        ],
        mappings: [
            phoneme(phoneme: "ph_s", glyph: "gt_s"),
            // sequence(phonemes: ["ph_k", "ph_a"], glyph: "g_ka")   — a syllabary sign
            // concept(concept: "STAR", glyph: "ge_star")            — a logogram
        ],
    ),
]
```

Three things worth knowing:

- **A glyph has an id, not just a shape.** `form` is how it is drawn *today*; `id` is
  what it *is*. Redraw every sign in a script and the mapping is untouched — which is
  what M21's glyph descent will hang from, and what would be impossible if the
  character were the identity.
- **The `kind` does not decide the mapping** — you declare that, sign by sign. What
  the kind decides is what counts as *expected*. An abjad with no vowel signs is doing
  its job; an alphabet with none has a hole, and the two print differently.
- **Longest match wins.** A `ka` sign beats a `k` sign followed by an `a` sign, by
  length rather than by declaration order — the same rule `Root::parse` uses.

> **The mapping is reported, never enforced.** A script that cannot write five of its
> language's fifteen sounds is a Note, not an error, and the file stays valid. §17
> again: unusual is not broken.
>
> **A logogram is keyed by `ConceptKey`, never by `WordId`.** `w_0080` means whatever
> the concept list holds at position 80, so an append to the built-in list would
> silently repoint every sign. The M14 rule, and it applies to scripts too.
>
> **Morphographic mapping is unbuilt.** §7.6 mentions it and it is real — Chinese
> radicals, Egyptian determinatives — but it needs a morpheme to point at, and this
> project's morphemes are language-scoped citation forms rather than the shared
> components a determinative system uses. It waits for a milestone that has one.

---

## Let the spelling fall behind

A script is a second history running alongside the language, and the two come apart.
That is not a defect in either of them — it is how every orthography you have ever
read became the way it is. The `gh` in *night* writes a sound English stopped making
six hundred years ago; there is no letter at all for the sound in the middle of
*measure*. Both directions, in one language.

Stemma models the sign's history, and **measures** the gap.

### A sign's biography

Give a glyph a `history:` — oldest first — and `stemma glyph-trace` walks it:

```bash
stemma glyph-trace out/written.ron ge_star
```

```
★  ge_star — the star
  The Emmen signs (logography)

  0  pictogram      ✶    = STAR
                  Six scratches from a common centre. A drawing of the thing.
  1  logogram       ✶    = STAR
                  The same shape, but now a WORD rather than a picture.
  2  determinative  ✶    (neither sound nor meaning)
                  Silent. Written before a god's name, and never read aloud.
  3  rebus sign     ✶    /s/
                  The hinge: borrowed for the SOUND its old word began with.
  4  phonogram      ★    /s/
                  Three strokes, written fast with a reed.
  →  now            ★    = STAR
```

That is §7.6's worked chain, and the rungs are the real ones: `pictogram`, `logogram`,
`determinative`, `rebus`, `phonogram`, `marker`. The **rebus** stage is the hinge of
the whole history of writing — the moment a picture of a star stops meaning *star* and
starts spelling the sound the word began with, after which a script can write words
nobody has drawn a picture for.

Three things to know about the shape of it:

- **Oldest first**, so reading down the page is reading forwards in time.
- **The present is not a stage.** `form` on the glyph is how it is drawn today and the
  script's `mappings` say what it does today; a stage repeating either would be a
  second source of truth. Notice above that the last *recorded* job is a letter for
  /s/ while the sign's job *now* is logographic — they differ, and only one is stored.
- **No stage carries a year.** A biography is authored prose the engine cannot check,
  and a date would dress an assertion up as a measurement. The order is the claim.

### And then the language moves

```bash
stemma apply-rules out/written.ron --rules fixtures/rules_glide_loss.sc \
    --id asterian_late --name "Late Written Asterian" --years 450 --out out/written_late.ron
stemma scripts out/written_late.ron
```

```
  The Kirran alphabet  (kirran, alphabet)
    15 sign(s)
    the language moved on: no sign for /b/ /d/ /ɡ/ (125 word(s) affected)
    signs that outlived their sound: w y
```

Two ordinary sound changes did that. Glide loss deleted every /w/ and /j/; intervocalic
voicing minted /b/, /d/ and /ɡ/. **Neither of them has ever heard of the script** —
open `fixtures/rules_glide_loss.sc` and you will find feature bundles and an
environment, the same as every rule since M3. They would run identically on a language
nobody had written down.

So ask the stranded letter what happened to it:

```bash
stemma glyph-trace out/written_late.ron g_w --script kirran
```

```
w  g_w — the hook
  0  pictogram   ʖ    = WATER
  1  rebus sign  ʖ    /w/
  →  now         w    /w/

  This sign has outlived its sound: no word of Late Written Asterian contains what it
  writes any more, and it is still on the page. It is recorded writing /w/ in the past.
```

### It is measured, not declared

Nothing in any file says "the script fell behind". Stemma reads the **word list** and
finds it — and it has to be the word list, because a sound change only ever *adds* to
the inventory. `/w/` is still sitting in the daughter's phoneme inventory; it is in no
word, and that is the question that matters.

Both halves show up in the profile, as §17's script-history row:

```bash
stemma profile out/written_late.ron
```

```
  Script history              historical  (5 mismatch(es): kirran 5, tirran 5, emmen 0)
```

That row was the last of M7's deferred dimensions but one. It reports, it does not
police — English is deep and is not thereby broken, and no amount of drift is ever an
error.

> **A logography cannot fall behind.** It writes meanings, so there is no pronunciation
> in it to drift from — which is exactly why logographic scripts outlive the
> pronunciations around them. Stemma says that rather than crediting the script with
> keeping up:
>
> ```
>   Script history   sound-independent  (writes meanings; no pronunciation to drift from)
> ```
>
> **Cross-script ancestry is unbuilt.** A sign descending from a sign in *another*
> script — Latin `A` from Phoenician *aleph* — is a contact fact between two writing
> systems, and faking one without a donor script in the file is the unsupported claim
> M15 removed from vocabulary. M21 builds the same-sign chain §7.6 describes.

---

## Let a model propose something

Stemma will take a suggestion from a language model. It will not take its word for
anything.

The rule is older than the feature: **no model output may change a language without
going through validation**, and it has been enforced since the first commit, when
there was nothing to enforce it against. What that buys you now is simple to state — a
model writes the same kind of file you would, and Stemma runs it through the same code
path your own file goes through, or refuses it.

There is no model inside Stemma. No API key, no network call, nothing to configure and
nothing that can bill you. You take a briefing to whichever assistant you like, and you
bring back a file.

### 1. Get the briefing

```bash
stemma brief fixtures/proto_asterian.ron --for rules
```

It prints the whole inventory with every feature bundle, the rules that have already
run, the exact syntax an artefact must be written in, and — usefully — the list of
mistakes that are refused outright. All of it is generated from the language file, so
it cannot be stale, and two runs are byte-identical.

Paste it into a model. Ask for sound changes.

### 2. Read what comes back

A proposal is an envelope with three parts, and they stay apart:

```
(
    id: "raising_and_voicing",
    target: "proto_asterian",
    provenance: (author: "claude-opus-5", method: "from `stemma brief`"),
    rationale: "Two changes: one that destroys a contrast and one that creates one. …",
    artefact: rules(( id: "rules_raising", name: "…", rules: [ … ] )),
)
```

- **`artefact`** is the only thing the engine ever sees. It is an ordinary `RuleSet` —
  the same thing as `fixtures/rules_asterian.ron`, with no field marking it as
  machine-written, because the engine must not be able to tell.
- **`rationale`** is the model's argument. Never parsed, never matched, never written
  into your language.
- **`provenance`** is for you, not for the code.

That separation is the point. Without the envelope, a model's reasoning ends up in a
`note:` field *inside* the rule set — stored in your language file, indistinguishable
from your own notes.

### 3. Review it

```bash
stemma review out/proto.ron --proposal fixtures/proposal_raising.ron
```

```
Proposal `raising_and_voicing` — for proto_asterian
  `rules_raising` — 2 rule(s)  ·  a sound-change rule set
  written by claude-opus-5 (from `stemma brief`)

  ── the proposer's own words, not the engine's ──
  │ Two changes: one that destroys a contrast and one that creates one.
  │ …
  ── nothing above was parsed, matched, or stored ──

  accepted — `rules_raising` — 2 rule(s) applies cleanly and would produce a traced,
  reproducible language
```

The prose is fenced and attributed, above the verdict — in the order it happened:
somebody claimed something, then the engine checked. `review` writes nothing; it runs
the *real* application against a throwaway copy and throws the result away, so its
verdict can never differ from what accepting would do.

### 4. Accept it

```bash
stemma accept out/proto.ron --proposal fixtures/proposal_raising.ron \
    --id raised --name "Raised Asterian" --years 450 --out out/raised.ron
```

The result is an ordinary daughter. Every word carries its derivation, `stemma trace`
walks it, and the file is byte-identical to what you would have got by typing those
rules yourself and running `apply-rules` — which is a test, not a claim.

### When it is wrong

`fixtures/proposal_incoherent.ron` is a real model proposal that gets refused, and it
is worth reading because of *how* it is wrong.

Its rationale is correct linguistics. Nasal place assimilation is real, it is the
design document's own example, and the file describes it accurately: a nasal takes the
place of a following stop, which is why everyone says *imput*. Nothing you could read
would tell you anything is wrong.

The rule copies place from `after[0]`, and its environment declares no `after` slot.
One missing line, in a file where every other line is right.

```bash
stemma review out/proto.ron --proposal fixtures/proposal_incoherent.ron
```

```
  refused — the rule set `rules_assimilation` cannot be applied; nothing was applied

  error: [rules.change_references_unmatched_position] the change copies from After(0),
  but the environment declares no slot there (r_a01)
```

Nothing is written. And the refusal is **whole** — that set's second rule is perfectly
good and does not run either, because a language carrying half a proposed history is
one nobody can reason about.

> **The model is not in the process, and that is the enforcement.** Stemma has no HTTP
> client and no API key. A model runs wherever you run it; the interface is a file you
> can read before you apply it.
>
> **Nothing in a language records that a model wrote it.** There is no `proposed_by`
> field and there will not be one: an accepted rule set is the same artefact whichever
> hand wrote it, and marking it otherwise would give the engine a distinction it must
> not have. Keep the proposal file if you want the record — it is yours, like every
> other file here.
>
> **Free translation is unbuilt.** `stemma say` puts a proposition through the formal
> grammar; a model translating free text would be inventing surface forms the engine
> never made, which is the one thing that must not happen.

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
| `grammar` | how this language builds a clause (§7.4's parameters) |
| `say` | put a proposition through a language and get a sentence |
| `shift` `shifts` | syntactic change, and the sound change that caused it |
| `scripts` `write` | writing systems, and what a spelling does and does not carry |
| `glyph-trace` | a sign's biography, and whether the sound it writes is still spoken |
| `brief` `review` `accept` | let a model propose an artefact, and gate it through the engine |
| `set-gloss` `add-word` `remove-word` `declare-concept` `reorder-rule` | edit a language |
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
| **Grammaticalization beyond case** — a topic marker cannot become an article, a serial verb cannot become an auxiliary, because none of those categories exists in the model. | case erosion forcing word order, with the causal chain checked (M19) | a later milestone, once there is a category to become |
| **Cross-script ancestry** — a sign descending from a sign in *another* script (Latin `A` from Phoenician *aleph*). | a sign's own chain, pictogram to present (M21) | a later milestone, once a donor script can be named in the file |
| **Morphographic writing** — a sign for a morpheme (Chinese radicals, Egyptian determinatives). | signs map from sounds, or from meanings | a milestone with shared morpheme components to point at |
| **Alien modality** — §7.7's pulse/scent/gesture languages. | the data model assumes a vocal tract throughout | **M23** embodiment profile → **M24** non-vocal signal systems |
| **A model inside Stemma** — no HTTP client, no API key, no `--model` flag. | a briefing out and a proposal in; the model runs wherever you run it (M22) | not planned: the out-of-process boundary *is* how the constraint is enforced, and an in-process client would make the test suite network-dependent |
| **Free translation** (§6.5's last bullet). | `say` puts a proposition through the formal grammar | a later milestone, if it can be done without inventing surface forms the engine never made |

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
