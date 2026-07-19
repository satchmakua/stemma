//! Seeded, portable, reproducible randomness.
//!
//! `DESIGN.md` §9.4 makes byte-for-byte reproducibility a hard constraint and
//! `CLAUDE.md` restates it: the same seed must reproduce the same language. That
//! constraint is measured in years, so this module refuses every convenience that
//! trades stability for ergonomics.
//!
//! This lives in `stem_core` because randomness is domain-free — it knows nothing
//! about linguistics, which is the entry condition for this crate — and because
//! M2's lexicon and M3's rule application will need the identical discipline.
//!
//! # Why ChaCha20 and not the standard generator
//!
//! `rand`'s standard generator is documented as *non-portable*: even with a fixed
//! seed its output is not portable, and as of rand 0.10 non-portable items may
//! make value-breaking changes in a *patch* release. A `cargo update` could
//! silently rewrite every stored language. ChaCha is a fixed published algorithm
//! with public test vectors; its keystream cannot drift.
//!
//! The workspace manifest sets `default-features = false` on rand, which removes
//! the non-portable generators from the API entirely. That, rather than any lint,
//! is what makes reaching for one impossible.
//!
//! # Why SHA-256 and not `seed_from_u64`
//!
//! `seed_from_u64` is a `rand_core`-provided default whose own documentation says
//! changing it should be considered a value-breaking change — an acknowledgement
//! of the risk, not a guarantee against it. SHA-256 is frozen by FIPS 180-4 and
//! can never change value under any crate version, which moves the `u64` to
//! `[u8; 32]` step entirely outside the dependency-stability question. (Rust's
//! `DefaultHasher` is the same trap one layer down and is explicitly not stable
//! across releases.)

use rand::SeedableRng;
use sha2::{Digest, Sha256};

/// The engine's random number generator.
///
/// Re-exported under a Stemma name so call sites read as intent rather than as a
/// library choice, and so a future deliberate change has exactly one place to
/// happen.
pub use rand::rngs::ChaCha20Rng as StemmaRng;

/// The versioned domain tag mixed into every seed.
///
/// Bumping this invalidates every seed in every existing project simultaneously.
/// There is no per-file opt-out and M1 does not build one — the tag exists to make
/// a deliberate future change to generation *possible and recorded*, not to make
/// it cheap. Change it only alongside a `PROGRESS.md` entry saying why.
const SEED_DOMAIN_VERSION: &[u8] = b"stemma/v1\0";

/// Which subsystem is drawing.
///
/// **Closed on purpose**, for the same reason [`crate::validate`] codes are stable
/// strings: a free-form `&str` domain cannot tell a new subsystem from a
/// misspelled one. `rng_for(seed, "root")` would compile, run, produce a perfectly
/// reproducible stream, and silently be a different language from `"roots"` — in
/// the one module whose entire job is determinism.
///
/// Each variant is an independent stream, so adding a subsystem later cannot
/// perturb an existing one's draws. That is what keeps an M2 lexicon from silently
/// changing every M1 root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RngDomain {
    /// Root generation (M1).
    Roots,
}

impl RngDomain {
    /// The frozen string hashed into the seed.
    ///
    /// These bytes are part of the determinism contract. Renaming a variant is
    /// free; changing a tag rewrites every language that uses it.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Roots => "roots",
        }
    }
}

/// Expands a user-facing `u64` seed into the generator's full 32-byte seed, with
/// domain separation.
fn expand_seed(seed: u64, domain: RngDomain) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SEED_DOMAIN_VERSION);
    hasher.update(domain.tag().as_bytes());
    hasher.update(b"\0");
    // `to_le_bytes`, never `to_ne_bytes`: endianness must be pinned, or the same
    // seed produces a different language on a big-endian target.
    hasher.update(seed.to_le_bytes());
    hasher.finalize().into()
}

/// A generator for one subsystem, seeded from the project seed.
pub fn rng_for(seed: u64, domain: RngDomain) -> StemmaRng {
    StemmaRng::from_seed(expand_seed(seed, domain))
}

#[cfg(test)]
mod tests {
    use super::*;
    // rand 0.10 renamed the traits: `Rng` is now the core trait (`next_u64`,
    // `fill_bytes`) and `RngExt` carries `.random()`. In 0.9 these were `RngCore`
    // and `Rng` respectively.
    use rand::Rng;
    use rand::RngExt;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Frozen. This is the value the whole determinism story reduces to: if it
    /// changes, every stored language changed. SHA-256 cannot drift, so the only
    /// way to break it is to edit `SEED_DOMAIN_VERSION` or [`RngDomain::tag`].
    #[test]
    fn seed_expansion_is_frozen_and_endianness_independent() {
        assert_eq!(
            hex(&expand_seed(0, RngDomain::Roots)),
            "c353e74033fd1cf6329b86660c3e0b4ea2579a1915086f1613bb51b0223cfd7e"
        );
        assert_eq!(
            hex(&expand_seed(42, RngDomain::Roots)),
            "7a5dfc56e0b28afbbd786834e22f680165eeeb929d131bb45292d40ebda0cfd3"
        );
    }

    /// The data-free dependency tripwire. No fixture edit can move this, so a red
    /// run here means the *generator itself* changed — which is the one thing the
    /// corpus digest cannot tell you unambiguously.
    #[test]
    fn the_raw_chacha_stream_for_seed_zero_is_frozen() {
        let mut rng = rng_for(0, RngDomain::Roots);
        let words: Vec<u64> = (0..8).map(|_| rng.random::<u64>()).collect();
        assert_eq!(
            words,
            [
                2042681168998186609,
                17503646098822309435,
                8096587704096398556,
                16968335496415568944,
                1732310318939720794,
                18359658017480799280,
                3873380661635047708,
                15560540372929170237,
            ]
        );
    }

    #[test]
    fn four_kilobytes_of_stream_hash_to_a_frozen_digest() {
        let mut rng = rng_for(0, RngDomain::Roots);
        let mut buffer = [0u8; 4096];
        rng.fill_bytes(&mut buffer);
        let digest: [u8; 32] = Sha256::digest(buffer).into();
        assert_eq!(
            hex(&digest),
            "03f3c472190f67b18aeee54e6b290b8cdd6e0fb7020a022ca8f6edff7cab0c76"
        );
    }

    #[test]
    fn the_same_seed_and_domain_produce_the_same_stream() {
        let draw = || -> Vec<u64> {
            let mut rng = rng_for(7, RngDomain::Roots);
            (0..32).map(|_| rng.random::<u64>()).collect()
        };
        assert_eq!(draw(), draw());
    }

    #[test]
    fn a_different_seed_produces_a_different_stream() {
        let draw = |seed| -> Vec<u64> {
            let mut rng = rng_for(seed, RngDomain::Roots);
            (0..16).map(|_| rng.random::<u64>()).collect()
        };
        assert_ne!(draw(1), draw(2));
    }

    /// Every tag is asserted literally, because these strings are hashed into
    /// seeds. A "harmless" rename is a silent break of every stored language.
    #[test]
    fn every_rng_domain_tag_is_frozen() {
        assert_eq!(RngDomain::Roots.tag(), "roots");
    }
}
