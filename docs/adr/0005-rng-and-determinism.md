# 5. ChaCha20 with SHA-256 seed expansion, and exact dependency pins

- **Status:** Accepted
- **Date:** 2026-07-19

## Context

`DESIGN.md` §9.4 makes byte-for-byte reproducibility a hard constraint: the same
seed and rule sequence must reproduce the same language. That promise is measured
in **years** — a user's saved project must still reproduce after unrelated
dependency upgrades — which makes it a supply-chain question, not just a coding
one.

The obvious implementation is `rand`'s standard generator seeded with
`seed_from_u64`. Investigation of the current crates found three separate ways
that silently breaks:

1. **`StdRng` is documented as non-portable** — even with a fixed seed its output
   is not portable, and as of rand 0.10 non-portable items may make value-breaking
   changes in a *patch* release. A routine `cargo update` could rewrite every
   stored language with no error and no diff.
2. **`seed_from_u64` is a `rand_core`-provided default** whose own documentation
   says changing it should be considered a value-breaking change — an
   acknowledgement of the risk, not a guarantee against it. (Rust's
   `DefaultHasher` is the same trap one layer down, and is explicitly not stable
   across releases.)
3. **`WeightedIndex` samples `Uniform` internally**, and rand 0.9.0's changelog
   reads verbatim: "Optimize distribution `Uniform` … (breaks value stability)".
   A caret version range would have shipped exactly that.

## Decision

- **ChaCha20** (`rand`'s `chacha` feature), via `from_seed`. A fixed published
  algorithm with public test vectors; its keystream cannot drift.
- **SHA-256 seed expansion.** `u64` seed → `[u8; 32]` via
  `SHA-256(b"stemma/v1\0" || domain_tag || b"\0" || seed.to_le_bytes())`. SHA-256
  is frozen by FIPS 180-4, which moves this step entirely outside the
  dependency-stability question. `to_le_bytes`, never `to_ne_bytes`, so endianness
  is pinned.
- **A closed `RngDomain` enum**, not a `&str`. A free-form domain cannot tell a new
  subsystem from a misspelled one: `rng_for(s, "root")` vs `"roots"` compiles,
  runs, is perfectly reproducible, and is silently a different language — in the
  one module whose entire job is determinism. Each variant is an independent
  stream, so adding a subsystem later cannot perturb an existing one's draws.
- **Exact `=` version pins** on `rand` and `sha2`, with a committed `Cargo.lock`.
- **`default-features = false`**, which removes `StdRng` and `ThreadRng` from the
  API entirely. This, not a lint, is the real enforcement: nobody can reach for the
  non-portable generator even by accident.
- **Integer (`u32`) weights, never floats.** The weighted sampler builds a
  prefix-sum array in iterator order; with floats, mathematically identical weight
  sets draw differently depending on summation order, and quantisation can silently
  zero a small weight — leaving a phoneme in the inventory and absent from every
  generated word, with no diagnostic.
- **`Vec` everywhere order reaches a distribution.** Never a `HashMap`, whose
  iteration order is randomised per process.

## Consequences

- Reproducibility is a property of the algorithm and the lockfile rather than a
  hope about upstream release discipline.
- **Three data-free canary tests** pin the seed expansion, the raw keystream, and
  the weighted-index draw sequence. No fixture edit can move them, so a red canary
  means the *generator* changed — distinguishable from the corpus golden digest,
  which legitimately moves when fixture content is edited. Having both is what
  keeps re-baselining the corpus digest an honest act rather than a habit.
- Upgrading `rand` becomes a deliberate act with a visible test failure, which is
  the point.
- **Authored order is part of the public contract.** Reversing an inventory and
  remapping indices back changes nearly every draw, because prefix-sum boundaries
  land at different points. A test asserts this dependency explicitly rather than
  leaving it as folklore. Any refactor that reorders an inventory, iterates a map,
  or changes a sort's tie-breaking rewrites every generated language.
- `SEED_DOMAIN_VERSION` is a **global** switch with no per-file opt-out. Bumping it
  invalidates every seed in every project at once, which is why it is never bumped
  except as a deliberate, `PROGRESS.md`-recorded break. A per-file
  `engine_seed_version` was considered and rejected: a field with exactly one legal
  value is scaffolding, and adding it later is a one-line `#[serde(default)]`.
