//! Three-tier symbol resolution and minting (`M3-SPEC` §4.1).
//!
//! A change produces a feature bundle; a stored form needs a [`PhonemeId`] and an
//! export needs an IPA string. This module is the only place the two meet.
//!
//! | Tier | Test | Outcome |
//! |---|---|---|
//! | 1 | bundle `==` a phoneme of the **input** inventory | `Inventory` |
//! | 2 | bundle `==` a `REFERENCE_SEGMENTS` row | `Innovated` — mint |
//! | 3 | neither | the site does not apply, at Warning severity |
//!
//! Exact match at every tier, `None` rather than an approximation, forever: on
//! the reference fixture `/k/[+voice]` sits at Hamming distance **1 from /k/
//! itself**, so a nearest-neighbour resolver would return the input segment —
//! silently undoing the rule while the trace asserts it applied.
//!
//! Tier 1 is evaluated against **the inventory as the run received it**, never
//! the growing one, so `Inventory` versus `Innovated` is a property of the
//! segment rather than of which word reached it first — reordering the lexicon
//! cannot flip a trace.

use stem_core::PhonemeId;
use stem_lexicon::{BlockReason, SymbolResolution};
use stem_phonology::reference::{self, ReferenceSegment};
use stem_phonology::{FeatureBundle, Phoneme, PhonemeInventory, required_features_missing};

/// One minted phoneme, pending its append to the output inventory.
#[derive(Debug, Clone)]
pub(crate) struct Minted {
    /// The table row that named it. Its index orders the final append.
    pub row_index: usize,
    /// The phoneme, fully built.
    pub phoneme: Phoneme,
}

/// The resolution state for one `apply_rules` run.
#[derive(Debug)]
pub(crate) struct Resolver<'a> {
    /// The inventory as the run received it. Never mutated.
    input: &'a PhonemeInventory,
    /// Mints so far, in discovery order. Appended to the output inventory in
    /// **table order**, so the evolved inventory is a function of the set of
    /// innovations rather than of lexicon traversal.
    minted: Vec<Minted>,
}

impl<'a> Resolver<'a> {
    pub(crate) fn new(input: &'a PhonemeInventory) -> Self {
        Self {
            input,
            minted: Vec::new(),
        }
    }

    /// The phoneme behind an id — input inventory first, then mints.
    ///
    /// This is the *matching* view: a segment minted by an earlier rule must be
    /// visible to a later rule's matcher, or feeding order is broken — which is
    /// the one thing ordered sound change is for.
    pub(crate) fn phoneme(&self, id: &PhonemeId) -> Option<&Phoneme> {
        self.input
            .get(id)
            .or_else(|| self.minted.iter().map(|m| &m.phoneme).find(|p| &p.id == id))
    }

    /// Resolves an output bundle to a symbol, minting if the reference table
    /// names it and nothing stands in the way.
    ///
    /// `source_weight` seeds a minted phoneme's `frequency_weight`. Inherited
    /// from the segment the rule transformed — a documented estimate, non-zero by
    /// construction because the source passed `bad_weight` validation
    /// (`M3-SPEC` §13.7). When several sources converge on one row, the mint
    /// keeps the maximum of their weights, so the evolved inventory's *contents*
    /// — not just its order — are independent of lexicon traversal.
    pub(crate) fn resolve(
        &mut self,
        bundle: FeatureBundle,
        source_weight: u32,
    ) -> Result<SymbolResolution, BlockReason> {
        // Tier 1: the author declared this segment. First in authored order, and
        // the rest recorded rather than hidden — first-declared-wins with no
        // diagnostic is Lexurgy issue #9.
        let declared = self.input.all_by_bundle(bundle);
        if let Some((first, rest)) = declared.split_first() {
            return Ok(SymbolResolution::Inventory {
                phoneme: first.id.clone(),
                ambiguous_with: rest.iter().map(|p| p.id.clone()).collect(),
            });
        }

        // The change wrote something the validator would reject. Same function the
        // validator uses, so the two cannot disagree.
        let missing = required_features_missing(bundle);
        if !missing.is_empty() {
            return Err(BlockReason::IllFormed {
                missing: missing.iter().map(|f| f.name().to_owned()).collect(),
            });
        }

        // Tier 2: the reference table names it.
        let Some(row) = reference::lookup(bundle) else {
            // Tier 3: refuse, with the full bundle so the fix is one reviewed
            // table row. Refusing rather than erroring or emitting U+FFFD: erroring
            // polices the creator (§17), and a replacement character destroys the
            // information *and* makes the trace a lie about an ancestor form.
            return Err(BlockReason::Unnameable {
                bundle: bundle.render(),
            });
        };

        // Already minted this run? Same row, same phoneme — two rules converging
        // on /ɡ/ produce one phoneme, which is the linguistically correct merger.
        if let Some(minted) = self
            .minted
            .iter_mut()
            .find(|m| std::ptr::eq(row_of(m.row_index), row))
        {
            // Convergent sources may carry different weights, and "whichever word
            // came first" would make the minted weight a function of lexicon
            // traversal — the exact dependence this module promises away. The
            // mint keeps the **maximum** over every source that fed it: max is a
            // function of the set, so word order stays unobservable.
            minted.phoneme.frequency_weight = minted.phoneme.frequency_weight.max(source_weight);
            return Ok(SymbolResolution::Innovated {
                phoneme: minted.phoneme.id.clone(),
                ipa: minted.phoneme.ipa.clone(),
            });
        }

        // The glyph must be free: minting a symbol the language already writes
        // would manufacture `duplicate_ipa`, an Error. (The holder necessarily has
        // a different bundle, or tier 1 would have hit.)
        if let Some(holder) = self.input.by_ipa(row.ipa) {
            return Err(BlockReason::SymbolHeld {
                ipa: row.ipa.to_owned(),
                by: holder.id.clone(),
            });
        }

        // The id must be free too. **No `_2` suffixing** — a suffix would make the
        // id a function of what else the language declares, and two sister
        // languages would then disagree about /ɡ/. Refusing keeps the id a pure
        // function of the segment.
        let id = PhonemeId::new(format!("ph_{}", row.slug));
        if self.input.get(&id).is_some() {
            return Err(BlockReason::IdHeld { id });
        }

        let mut phoneme = Phoneme::new(id.clone(), row.ipa, row.kind)
            .with_weight(source_weight)
            .with_features(bundle);
        if let Some(romanization) = row.romanization {
            phoneme = phoneme.with_romanization(romanization);
        }

        self.minted.push(Minted {
            row_index: reference::row_index(row),
            phoneme,
        });

        Ok(SymbolResolution::Innovated {
            phoneme: id,
            ipa: row.ipa.to_owned(),
        })
    }

    /// The output inventory: the input unchanged, then every mint in
    /// **reference-table order**.
    pub(crate) fn into_inventory(mut self) -> (PhonemeInventory, Vec<Phoneme>) {
        self.minted.sort_by_key(|m| m.row_index);
        let mut inventory = self.input.clone();
        let minted: Vec<Phoneme> = self.minted.into_iter().map(|m| m.phoneme).collect();
        for phoneme in &minted {
            inventory.push(phoneme.clone());
        }
        (inventory, minted)
    }
}

fn row_of(index: usize) -> &'static ReferenceSegment {
    &reference::REFERENCE_SEGMENTS[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two sources with different weights converging on one reference row must
    /// yield the same minted weight in either encounter order — otherwise the
    /// evolved genome's bytes depend on which word the lexicon lists first,
    /// which the module doc promises cannot happen.
    #[test]
    fn a_convergent_mint_keeps_the_same_weight_in_either_encounter_order() {
        let row = &reference::REFERENCE_SEGMENTS[0];
        let bundle = reference::bundle_of(row);
        let input = PhonemeInventory::from_phonemes(Vec::<Phoneme>::new());

        let weight_after = |weights: &[u32]| {
            let mut resolver = Resolver::new(&input);
            for &w in weights {
                resolver.resolve(bundle, w).expect("a table row mints");
            }
            let (_, minted) = resolver.into_inventory();
            minted[0].frequency_weight
        };

        let forward = weight_after(&[30, 45]);
        let reversed = weight_after(&[45, 30]);
        assert_eq!(
            forward, reversed,
            "minted weight depends on encounter order: {forward} vs {reversed}"
        );
        assert_eq!(forward, 45, "the mint keeps the maximum source weight");
    }
}
