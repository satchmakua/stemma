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

**Status:** early — **M0 (walking skeleton) complete**. The workspace builds, the
CLI loads and validates real language files, and 51 tests pass. The diachronic
engine itself (root generation, sound change, forking, tracing) is Phase 1 work.
See [ROADMAP.md](ROADMAP.md) for the plan and [PROGRESS.md](PROGRESS.md) for what
has shipped.

---

## Run it

**Prerequisites:** Rust ≥ 1.85 with the 2024 edition (check: `cargo --version`).
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
✗ phonology.bad_weight: frequency weight must be finite and positive, got -2 (ph_k)
✗ phonology.duplicate_ipa: 2 phonemes share the IPA form /t/; they are not contrastive (t)
✗ phonology.no_nucleus: no phoneme can be a syllable nucleus, so no syllable can be formed
· phonology.very_small_inventory: 3 phonemes is smaller than any attested language
```

Note the grading: `✗` blocks the pipeline, `!` is a typological warning, `·` is a
note. Unusual languages are flagged, not rejected — the tool guides the creator
rather than policing them.

### Commands

| Command | What it does |
|---|---|
| `stemma validate <file>` | Check a language for structural errors and typological oddities (exit 1 if invalid) |
| `stemma info <file>` | Print a language's inventory and lineage |
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
        (id: "ph_t", ipa: "t", kind: consonant, frequency_weight: 5.0),
        (id: "ph_a", ipa: "a", kind: vowel,     frequency_weight: 6.0),
    ],
)
```

Every stochastic step is seeded, so the same file plus the same rules always
produces the same descendants.

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
| [`docs/adr/`](docs/adr/) | Architecture decision records. |

## Tech stack

Rust (edition 2024) workspace of seven small crates — `stem_core` (IDs, errors,
validation), `stem_phonology`, `stem_lexicon`, `stem_soundchange`, `stem_genome`,
`stem_io`, and the `stemma` CLI. Serialisation via serde with RON as the authored
project format and JSON for interchange. No database: a language is a file.

A desktop/web UI is planned (M11) as a second front end onto the same crates —
deliberately not before the engine works.

## License

MIT — see [LICENSE](LICENSE).
