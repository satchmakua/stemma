# Growing a Language Family in 90 Seconds

One proto-language, three daughters, and roughly five centuries of simulated change. Every modern form below carries a complete, rule-derived causal history — nothing here was chosen at random. The whole document is deterministic: the same proto-language and the same three ordered rule files reproduce it byte-for-byte.

## The proto-language — Attested Asterian

9 attested roots, the ancestor every modern form below descends from. Stress is fixed on the first syllable — which is what makes "final *unstressed* vowel loss" a real conditioned rule rather than "drop the last vowel".

| Gloss | Form | IPA |
| --- | --- | --- |
| star | takala | /takala/ |
| moon | taka | /taka/ |
| sun | sawel | /sawel/ |
| water | akwa | /akwa/ |
| king | rekan | /rekan/ |
| stone | sanka | /sanka/ |
| mountain | anpa | /anpa/ |
| fire | amta | /amta/ |
| mother | mikala | /mikala/ |

The full dictionary — every form with its cognate set — is one command away: `stemma export-md <file>`.

## The daughters

Each branched off and underwent its own ordered sound-change history (dates are years since the proto-language):

### Coastal Asterian — +470y from Attested Asterian

The innovator. Intervocalic voicing feeds lenition feeds loss, and final unstressed vowels drop — so its velars hollow out entirely: *takala* becomes *taal*.

- Intervocalic voicing — at 120y
- Velar lenition — at 260y
- Intervocalic velar fricative loss — at 380y
- Final unstressed vowel loss — at 470y

### Highland Asterian — +460y from Attested Asterian

Shares voicing and apocope with the coast, but assimilates nasals instead of leniting, so its velars survive as stops: *takala* becomes *tagal*.

- Intervocalic voicing — at 120y
- Nasal place assimilation — at 300y
- Final unstressed vowel loss — at 460y

### Riverine Asterian — +420y from Attested Asterian

The conservative branch. It never voiced and never lost a final vowel; a velar-loss-then-coalescence chain of its own gives *takala* → *tala*, and it keeps the vowels its sisters drop.

- Intervocalic velar stop loss — at 150y
- Low vowel coalescence — at 300y
- Nasal place assimilation — at 420y

## Comparative cognate table

Reflexes of each meaning across the family, joined by **shared ancestry** (cognate set) rather than by meaning — so a reflex holds its row even if its sense later drifts on one branch.

| Meaning | Attested Asterian | Coastal Asterian | Highland Asterian | Riverine Asterian |
| --- | --- | --- | --- | --- |
| water | *akwa | akw | akw | akwa |
| sun | *sawel | sawel | sawel | sawel |
| star | *takala | taal | tagal | tala |
| stone | *sanka | sank | saŋk | saŋka |
| king | *rekan | rean | regan | rean |
| mother | *mikala | mial | migal | miala |

*Proto-forms (marked \*) are reconstructions.*

One etymon, `*takala`, takes three roads — `taal`, `tagal`, `tala`: the comparative method in a single row.

## Five etymologies

A strange-looking word is strange *for a reason*. Here is each one's full derivation, rule by rule — including a rule that could have fired but did not, so the trace explains a *skip* as well as a change.

### star — Coastal Asterian: *takala → taal

```text
*takala  "star"  cog_asterian_attested_0001

  proto      takala        /takala/
  │ stress   TA.ka.la
  │
  0  r_ivv  Intervocalic voicing
  │    k > g  [2,3)   environment  a _ a
  │    /ɡ/ is new to this language  (reference table)
  │    → tagala      /taɡala/
  │
  1  r_velar_lenition  Velar lenition
  │    g > ɣ  [2,3)   environment  a _ a
  │    /ɣ/ is new to this language  (reference table)
  │    → taɣala      /taɣala/
  │
  2  r_gamma_loss  Intervocalic velar fricative loss
  │    ɣ > ∅  [2,3)   environment  a _ a
  │    → taala       /taala/
  │
  3  r_apocope  Final unstressed vowel loss
  │    a > ∅  [4,5)   environment  l _ #
  │    → taal        /taal/
  │
  modern     taal          /taal/
```

### star — Riverine Asterian: *takala → tala

```text
*takala  "star"  cog_asterian_attested_0001

  proto      takala        /takala/
  │ stress   TA.ka.la
  │
  0  r_velar_loss  Intervocalic velar stop loss
  │    k > ∅  [2,3)   environment  a _ a
  │    → taala       /taala/
  │
  1  r_low_coalescence  Low vowel coalescence
  │    a > ∅  [2,3)   environment  a _ l
  │    a syllable (pattern CV) emptied and was removed
  │    → tala        /tala/
  │
  2  r_nasal_place  Nasal place assimilation — did not apply
  │
  modern     tala          /tala/
```

### stone — Highland Asterian: *sanka → saŋk

```text
*sanka  "stone"  cog_asterian_attested_0006

  proto      sanka         /sanka/
  │ stress   SAN.ka
  │
  0  r_ivv  Intervocalic voicing — did not apply
  │
  1  r_nasal_place  Nasal place assimilation
  │    n > ŋ  [2,3)   environment  a _ k
  │    /ŋ/ is new to this language  (reference table)
  │    → saŋka       /saŋka/
  │
  2  r_apocope  Final unstressed vowel loss
  │    a > ∅  [4,5)   environment  k _ #
  │    → saŋk        /saŋk/
  │
  modern     saŋk          /saŋk/
```

### king — Coastal Asterian: *rekan → rean

```text
*rekan  "king"  cog_asterian_attested_0005

  proto      rekan         /rekan/
  │ stress   RE.kan
  │
  0  r_ivv  Intervocalic voicing
  │    k > g  [2,3)   environment  e _ a
  │    /ɡ/ is new to this language  (reference table)
  │    → regan       /reɡan/
  │
  1  r_velar_lenition  Velar lenition
  │    g > ɣ  [2,3)   environment  e _ a
  │    /ɣ/ is new to this language  (reference table)
  │    → reɣan       /reɣan/
  │
  2  r_gamma_loss  Intervocalic velar fricative loss
  │    ɣ > ∅  [2,3)   environment  e _ a
  │    → rean        /rean/
  │
  3  r_apocope  Final unstressed vowel loss — did not apply
  │
  modern     rean          /rean/
```

### mother — Riverine Asterian: *mikala → miala

```text
*mikala  "mother"  cog_asterian_attested_0009

  proto      mikala        /mikala/
  │ stress   MI.ka.la
  │
  0  r_velar_loss  Intervocalic velar stop loss
  │    k > ∅  [2,3)   environment  i _ a
  │    → miala       /miala/
  │
  1  r_low_coalescence  Low vowel coalescence — did not apply
  │
  2  r_nasal_place  Nasal place assimilation — did not apply
  │
  modern     miala         /miala/
```

## What this demonstrates

Every surface form was produced by an ordered rule, not chosen; each etymology replays from proto to modern, step by step, and even a rule that found no target is on the record. The table joins by cognate set — shared ancestry — so a reflex keeps its row no matter what its meaning later does. And the whole document is a pure function of one proto-language and three rule files: same inputs, identical bytes.

Still ahead, and named honestly rather than faked: **meaning drift** (a reflex coming to mean something new on one branch while its sisters keep the old sense), **morphology and a grammar sketch**, a readable rule notation, and a visual explorer. For the full dictionary as Markdown or CLDF-shaped CSV, run `stemma export-md` or `stemma export-csv`.

---
*Generated by Stemma from a proto-language and an ordered rule set. Every form above is reproducible byte-for-byte from the same inputs — no wall-clock time, no unseeded randomness.*
