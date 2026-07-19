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

**Status:** early — **M1 complete**. Phonemes carry real distinctive features,
languages declare their syllable shapes, and the CLI generates reproducible root
words. 157 tests pass. Sound change, forking and tracing are still ahead. See
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

### Commands

| Command | What it does |
|---|---|
| `stemma validate <file>` | Check a language for structural errors and typological oddities (exit 1 if invalid) |
| `stemma info <file>` | Print a language's inventory, lineage, and root shapes |
| `stemma features <file>` | Show each phoneme's resolved feature matrix |
| `stemma generate-roots <file>` | Generate root words (`--count`, `--seed`, `--ipa`) |
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

Rust (edition 2024) workspace of seven small crates — `stem_core` (IDs, errors,
validation, seeded RNG), `stem_phonology` (features, inventory, phonotactics,
generation), `stem_lexicon`, `stem_soundchange`, `stem_genome`, `stem_io`, and the
`stemma` CLI. Serialisation via serde with RON as the authored project format and
JSON for interchange. Randomness is ChaCha20 with SHA-256 seed expansion and exact
dependency pins, because reproducibility here is measured in years. No database: a
language is a file.

A desktop/web UI is planned (M11) as a second front end onto the same crates —
deliberately not before the engine works.

## License

MIT — see [LICENSE](LICENSE).
