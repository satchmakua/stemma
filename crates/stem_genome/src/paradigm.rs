//! Rendering a paradigm as the regular→irregular table (M8, `DESIGN.md` §10.2,
//! §7.3).
//!
//! A **pure library renderer** — the `render_family` / `render_profile` precedent
//! (`docs/adr/0006`): built in memory, newline-terminated, no map, no float, no
//! clock, so two calls are byte-identical (§9.4), and the M11 UI renders the
//! identical text. It reads the genome's inflected cells and each cell's stored
//! `trace`; it runs no engine.
//!
//! # What it shows, and why it is the milestone made legible
//!
//! The stems×cells grid of **surface** forms, then — per affix — its distinct
//! surface allomorphs and, for each, *which sound change produced it*. On a proto
//! the plural suffix is one allomorph (`-ka`, regular); after intervocalic voicing
//! has run it is two (`-ɡa` after a vowel, `-ka` after a consonant, irregular), and
//! the table says the split was conditioned by that rule. That is ROADMAP M8's
//! acceptance — *a regular paradigm becomes irregular purely as a consequence of an
//! ordered sound change, and the trace explains why* — rendered. The per-cell full
//! derivation stays one command away: `stemma trace <cell-id>`.
//!
//! # The join is derived, never stored
//!
//! A cell is matched to its `WordEntry` by comparing morpheme ids — the entry whose
//! decomposition is `[stem] + cell.affixes`. There is no stored cell→word map; a
//! stored edge is the desync class `docs/adr/0008` bans. Which rule conditioned an
//! allomorph is likewise *derived*, by replaying the cell's own trace one step at a
//! time and seeing which step moved the affix's segments.

use std::fmt::Write;

use stem_core::{MorphemeId, PhonemeId, Result, RuleId, StemmaError};
use stem_lexicon::{Derivation, MorphemeRole, Paradigm, WordEntry};
use stem_phonology::PhonemeInventory;

use crate::LanguageGenome;

/// Renders `paradigm` over `genome`'s inflected lexicon.
///
/// Errors if a stem the paradigm names is absent from the morphology, or if a
/// `(stem, cell)` has no materialised word — i.e. the paradigm has not been
/// inflected into this language yet (run `stemma inflect` first).
pub fn render_paradigm(genome: &LanguageGenome, paradigm: &Paradigm) -> Result<String> {
    let inventory = &genome.phonemes;
    let mut out = String::new();

    writeln!(
        out,
        "Paradigm — {} ({})  ·  {}",
        paradigm.name, paradigm.id, genome.name
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    // Row labels: each stem's underlying citation form, romanised (`tira`).
    let mut row_labels: Vec<String> = Vec::with_capacity(paradigm.stems.len());
    for stem_id in &paradigm.stems {
        let stem = genome
            .morphology
            .morpheme(stem_id)
            .ok_or_else(|| StemmaError::not_found("stem morpheme", stem_id))?;
        row_labels.push(romanize(inventory, stem.form.segments())?);
    }

    // The surface grid: one cell per (stem, cell), matched by decomposition.
    // `grid[r][c]` is the romanised surface form; we also keep the entry for the
    // allomorphy pass so the join is computed once.
    let mut grid: Vec<Vec<String>> = Vec::with_capacity(paradigm.stems.len());
    let mut words: Vec<Vec<&WordEntry>> = Vec::with_capacity(paradigm.stems.len());
    for stem_id in &paradigm.stems {
        let mut surface_row = Vec::with_capacity(paradigm.cells.len());
        let mut word_row = Vec::with_capacity(paradigm.cells.len());
        for cell in &paradigm.cells {
            let entry = find_cell_word(genome, stem_id, &cell.affixes).ok_or_else(|| {
                StemmaError::not_found(
                    "inflected cell",
                    format!("{stem_id}/{} (run `stemma inflect` first)", cell.label),
                )
            })?;
            surface_row.push(entry.written(inventory)?);
            word_row.push(entry);
        }
        grid.push(surface_row);
        words.push(word_row);
    }

    render_grid(&mut out, paradigm, &row_labels, &grid)?;

    // The exponents section: per affix, its distinct allomorphs and each one's
    // conditioning rule (or "did not apply"), derived from the cells' own traces.
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "Exponents:").map_err(fmt_err)?;
    render_exponents(&mut out, genome, paradigm, &row_labels, &words)?;

    Ok(out)
}

/// One occurrence of an affix in a specific cell.
struct Occurrence {
    /// Which stem row it belongs to (its romanised label).
    stem_label: String,
    /// The affix's surface segments in this cell.
    surface: Vec<PhonemeId>,
    /// The rules whose application changed the affix's segments here — empty when
    /// no rule touched it (the affix surfaced unchanged).
    fired: Vec<RuleId>,
}

/// One affix's realisation across the paradigm, gathered in first-appearance order.
struct AffixReport {
    id: MorphemeId,
    gloss: String,
    underlying: Vec<PhonemeId>,
    occurrences: Vec<Occurrence>,
}

fn render_exponents(
    out: &mut String,
    genome: &LanguageGenome,
    paradigm: &Paradigm,
    row_labels: &[String],
    words: &[Vec<&WordEntry>],
) -> Result<()> {
    let inventory = &genome.phonemes;

    // Gather affix occurrences, preserving first-appearance order (never a map).
    let mut reports: Vec<AffixReport> = Vec::new();
    // A zero-exponent cell (bare stem) — announce it once, honestly.
    let mut has_zero_exponent = false;

    for (r, row) in words.iter().enumerate() {
        for (c, entry) in row.iter().enumerate() {
            let cell = &paradigm.cells[c];
            if cell.affixes.is_empty() {
                has_zero_exponent = true;
            }
            for reference in entry
                .morphemes
                .iter()
                .filter(|m| m.role != MorphemeRole::Stem)
            {
                let (start, end) = (reference.start as usize, reference.end as usize);
                // The shared `morpheme_surface` (stem_lexicon) — the same source the
                // allomorph measure reads, so the rendered count and the profile
                // band cannot disagree (`docs/adr/0009`).
                let surface = entry.morpheme_surface(start, end);
                let fired = match &entry.trace {
                    Some(trace) => conditioning_rules(trace, start, end),
                    None => Vec::new(),
                };
                let occ = Occurrence {
                    stem_label: row_labels[r].clone(),
                    surface,
                    fired,
                };
                match reports.iter_mut().find(|a| a.id == reference.morpheme) {
                    Some(a) => a.occurrences.push(occ),
                    None => {
                        let underlying = genome
                            .morphology
                            .morpheme(&reference.morpheme)
                            .map(|m| m.form.segments().cloned().collect())
                            .unwrap_or_default();
                        reports.push(AffixReport {
                            id: reference.morpheme.clone(),
                            gloss: reference.gloss.clone(),
                            underlying,
                            occurrences: vec![occ],
                        });
                    }
                }
            }
        }
    }

    if has_zero_exponent {
        writeln!(out, "  ∅   zero exponent (the bare stem)").map_err(fmt_err)?;
    }

    for affix in &reports {
        // Distinct allomorphs, first-seen order.
        let mut allomorphs: Vec<Vec<PhonemeId>> = Vec::new();
        for occ in &affix.occurrences {
            if !allomorphs.contains(&occ.surface) {
                allomorphs.push(occ.surface.clone());
            }
        }
        let count = allomorphs.len();
        let heading: Vec<String> = allomorphs
            .iter()
            .map(|a| romanize(inventory, a.iter()).map(|s| format!("-{s}")))
            .collect::<Result<Vec<_>>>()?;
        writeln!(
            out,
            "  {}   {}   — {count} allomorph{}{}",
            affix.gloss,
            heading.join(" / "),
            if count == 1 { "" } else { "s" },
            if count > 1 { " (irregular)" } else { "" },
        )
        .map_err(fmt_err)?;

        // One detail line per allomorph: which stems, and the conditioning rule.
        for allomorph in &allomorphs {
            let occs: Vec<&Occurrence> = affix
                .occurrences
                .iter()
                .filter(|o| &o.surface == allomorph)
                .collect();
            // Distinct stems, first-seen order: two syncretic cells resolve to one
            // entry, so the same stem can occur twice — list it once.
            let mut stems: Vec<&str> = Vec::new();
            for occ in &occs {
                if !stems.contains(&occ.stem_label.as_str()) {
                    stems.push(occ.stem_label.as_str());
                }
            }
            let rom = romanize(inventory, allomorph.iter())?;

            // Union of the rules that produced this allomorph, in encounter order.
            let mut fired: Vec<RuleId> = Vec::new();
            for occ in &occs {
                for rule in &occ.fired {
                    if !fired.contains(rule) {
                        fired.push(rule.clone());
                    }
                }
            }
            let note = if !fired.is_empty() {
                let names: Vec<String> = fired.iter().map(|id| rule_name(genome, id)).collect();
                format!("   ← {} fired", names.join(", "))
            } else if allomorph == &affix.underlying && count > 1 {
                // Unchanged, but a sister allomorph did change: name the contrast.
                "   ← the conditioning rule did not apply here".to_owned()
            } else {
                String::new()
            };
            writeln!(out, "    -{rom}   after {}{note}", stems.join(", ")).map_err(fmt_err)?;
        }
    }

    Ok(())
}

/// Which rules changed the segments in `[start, end)`, by replaying the trace one
/// step at a time and comparing the span before and after each step. Derived, so it
/// cannot disagree with the stored derivation.
fn conditioning_rules(trace: &Derivation, start: usize, end: usize) -> Vec<RuleId> {
    let mut rules = Vec::new();
    for k in 0..trace.steps.len() {
        let before = Derivation {
            input: trace.input.clone(),
            steps: trace.steps[..k].to_vec(),
        }
        .surface_of_input_span(start, end);
        let after = Derivation {
            input: trace.input.clone(),
            steps: trace.steps[..=k].to_vec(),
        }
        .surface_of_input_span(start, end);
        if before != after {
            rules.push(trace.steps[k].rule.clone());
        }
    }
    rules
}

/// A rule's display name from the genome's applied log, or its id if unrecorded.
fn rule_name(genome: &LanguageGenome, id: &RuleId) -> String {
    genome
        .applied_rules
        .iter()
        .find(|r| &r.id == id)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| id.to_string())
}

/// The `WordEntry` whose decomposition is exactly `[stem] + affixes` **in surface
/// order** — the derived join, no stored edge (`docs/adr/0008`).
///
/// Matching the ordered sequence, not just the multiset, keeps two cells that share
/// an affix *set* but differ in affix *order* — `x·y·stem` vs `y·x·stem`, which
/// `compose` materialises as different forms — from resolving to each other's form.
/// Two cells with the *same* ordered exponent are genuine syncretism: their forms
/// coincide, so resolving both to the first is correct for the grid (and the
/// exponent listing dedups the repeated stem).
fn find_cell_word<'a>(
    genome: &'a LanguageGenome,
    stem: &MorphemeId,
    affixes: &[MorphemeId],
) -> Option<&'a WordEntry> {
    let expected = expected_decomposition(genome, stem, affixes);
    genome.lexicon.iter().find(|entry| {
        entry
            .morphemes
            .iter()
            .map(|m| &m.morpheme)
            .eq(expected.iter())
    })
}

/// The surface-order morpheme-id sequence `compose` produces for `[stem] + affixes`:
/// prefixes (authored order) · stem · suffixes (authored order). Mirrors
/// `stem_lexicon::compose`'s ordering so the join matches what `inflect` stored.
fn expected_decomposition(
    genome: &LanguageGenome,
    stem: &MorphemeId,
    affixes: &[MorphemeId],
) -> Vec<MorphemeId> {
    let is_prefix = |id: &MorphemeId| {
        genome.morphology.morpheme(id).map(|m| m.role) == Some(MorphemeRole::Prefix)
    };
    let mut ids: Vec<MorphemeId> = affixes.iter().filter(|a| is_prefix(a)).cloned().collect();
    ids.push(stem.clone());
    ids.extend(affixes.iter().filter(|a| !is_prefix(a)).cloned());
    ids
}

/// Romanises a run of segments through the inventory. `Result` because a caller
/// could pass an inventory missing a segment (`Root::written`'s reasoning).
fn romanize<'a>(
    inventory: &PhonemeInventory,
    segments: impl Iterator<Item = &'a PhonemeId>,
) -> Result<String> {
    let mut out = String::new();
    for id in segments {
        out.push_str(inventory.require(id)?.written());
    }
    Ok(out)
}

/// Renders the aligned stems×cells grid of surface forms, padded by char count (for
/// the IPA glyphs), like `render_profile`.
fn render_grid(
    out: &mut String,
    paradigm: &Paradigm,
    row_labels: &[String],
    grid: &[Vec<String>],
) -> Result<()> {
    // Column 0 width: the widest row label.
    let label_width = row_labels
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    // Each data column's width: the widest of its header and its cells.
    let col_widths: Vec<usize> = paradigm
        .cells
        .iter()
        .enumerate()
        .map(|(c, cell)| {
            let header = cell.label.chars().count();
            let cells = grid
                .iter()
                .map(|row| row[c].chars().count())
                .max()
                .unwrap_or(0);
            header.max(cells)
        })
        .collect();

    // Header row.
    let mut header = format!("  {}", " ".repeat(label_width));
    for (c, cell) in paradigm.cells.iter().enumerate() {
        header.push_str(&format!(
            "   {}{}",
            cell.label,
            crate::pad(&cell.label, col_widths[c])
        ));
    }
    writeln!(out, "{}", header.trim_end()).map_err(fmt_err)?;

    // Data rows.
    for (r, label) in row_labels.iter().enumerate() {
        let mut line = format!("  {}{}", label, crate::pad(label, label_width));
        for (c, _) in paradigm.cells.iter().enumerate() {
            let form = &grid[r][c];
            line.push_str(&format!("   {}{}", form, crate::pad(form, col_widths[c])));
        }
        writeln!(out, "{}", line.trim_end()).map_err(fmt_err)?;
    }

    Ok(())
}

/// A `std::fmt::Error` in string building means an allocation failure, not a domain
/// error; surface it as a serialize error rather than panicking.
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "paradigm",
        message: "formatting a paradigm into a string failed".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_lexicon::{Morpheme, Morphology, ParadigmCell};

    // A tiny, engine-free genome: two stems (one vowel-final, one consonant-final),
    // a plural suffix, the NUMBER paradigm, inflected — then, for the "irregular"
    // tests, hand-authored traces stand in for a real rule run (the *engine*
    // verification is the CLI acceptance test over the real fixture).

    fn syllable(pattern: &str, segments: &[&str]) -> stem_phonology::Syllable {
        stem_phonology::Syllable {
            pattern: pattern.to_owned(),
            segments: segments.iter().map(|s| PhonemeId::new(*s)).collect(),
            stress: None,
        }
    }

    fn featured(
        id: &str,
        ipa: &str,
        kind: stem_phonology::SegmentKind,
        tokens: &[&str],
    ) -> stem_phonology::Phoneme {
        let bundle = stem_phonology::FeatureBundle::try_from(
            tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        )
        .expect("valid features");
        stem_phonology::Phoneme::new(id, ipa, kind).with_features(bundle)
    }

    fn inventory() -> PhonemeInventory {
        use stem_phonology::SegmentKind::{Consonant, Vowel};
        // Minimal features — enough to be a well-formed inventory and to romanise.
        let c = |id: &str, ipa: &str| {
            featured(
                id,
                ipa,
                Consonant,
                &[
                    "-syllabic",
                    "+consonantal",
                    "-sonorant",
                    "-approximant",
                    "-continuant",
                    "-nasal",
                    "-lateral",
                    "-trill",
                    "-voice",
                    "-labial",
                    "+coronal",
                    "-dorsal",
                ],
            )
        };
        let son = |id: &str, ipa: &str| {
            featured(
                id,
                ipa,
                Consonant,
                &[
                    "-syllabic",
                    "+consonantal",
                    "+sonorant",
                    "-approximant",
                    "-continuant",
                    "+nasal",
                    "-lateral",
                    "-trill",
                    "+voice",
                    "-labial",
                    "+coronal",
                    "-dorsal",
                ],
            )
        };
        let v = |id: &str, ipa: &str| {
            featured(
                id,
                ipa,
                Vowel,
                &[
                    "+syllabic",
                    "-consonantal",
                    "+sonorant",
                    "+approximant",
                    "+continuant",
                    "-nasal",
                    "-lateral",
                    "-trill",
                    "+voice",
                    "-labial",
                    "-coronal",
                    "+dorsal",
                    "-high",
                    "+low",
                    "+back",
                    "-round",
                ],
            )
        };
        PhonemeInventory::from_phonemes([
            c("ph_t", "t"),
            c("ph_k", "k"),
            son("ph_n", "n"),
            son("ph_r", "r"),
            v("ph_i", "i"),
            v("ph_a", "a"),
            // The voiced stop the "rule" innovates, declared so the evolved genome
            // can romanise it (the real pipeline mints it into the inventory).
            featured(
                "ph_g",
                "\u{0261}",
                Consonant,
                &[
                    "-syllabic",
                    "+consonantal",
                    "-sonorant",
                    "-approximant",
                    "-continuant",
                    "-nasal",
                    "-lateral",
                    "-trill",
                    "+voice",
                    "-labial",
                    "+coronal",
                    "-dorsal",
                ],
            ),
        ])
    }

    fn morphology() -> Morphology {
        let stem = |id: &str, gloss: &str, syls: Vec<stem_phonology::Syllable>| Morpheme {
            id: MorphemeId::new(id),
            role: MorphemeRole::Stem,
            gloss: gloss.to_owned(),
            form: stem_phonology::Root { syllables: syls },
            part_of_speech: stem_lexicon::PartOfSpeech::Noun,
        };
        Morphology {
            morphemes: vec![
                stem(
                    "m_tira",
                    "star",
                    vec![
                        syllable("CV", &["ph_t", "ph_i"]),
                        syllable("CV", &["ph_r", "ph_a"]),
                    ],
                ),
                stem(
                    "m_tan",
                    "man",
                    vec![syllable("CVC", &["ph_t", "ph_a", "ph_n"])],
                ),
                Morpheme {
                    id: MorphemeId::new("m_plural"),
                    role: MorphemeRole::Suffix,
                    gloss: "PL".to_owned(),
                    form: stem_phonology::Root {
                        syllables: vec![syllable("CV", &["ph_k", "ph_a"])],
                    },
                    part_of_speech: stem_lexicon::PartOfSpeech::Noun,
                },
            ],
            paradigms: vec![Paradigm {
                id: "NUMBER".to_owned(),
                name: "Number".to_owned(),
                stems: vec![MorphemeId::new("m_tira"), MorphemeId::new("m_tan")],
                cells: vec![
                    ParadigmCell {
                        label: "SG".to_owned(),
                        affixes: vec![],
                    },
                    ParadigmCell {
                        label: "PL".to_owned(),
                        affixes: vec![MorphemeId::new("m_plural")],
                    },
                ],
            }],
        }
    }

    fn proto() -> LanguageGenome {
        let morphology = morphology();
        let cells = stem_lexicon::inflect(
            &morphology.paradigms[0],
            &morphology.morphemes,
            &stem_core::LanguageId::new("proto_x"),
        )
        .expect("inflects");
        LanguageGenome::proto("proto_x", "Proto-X")
            .with_phonemes(inventory())
            .with_morphology(morphology)
            .with_lexicon(stem_lexicon::Lexicon::from_entries(cells))
    }

    #[test]
    fn a_proto_paradigm_renders_regular_with_one_allomorph() {
        let genome = proto();
        let paradigm = genome.morphology.paradigms[0].clone();
        let text = render_paradigm(&genome, &paradigm).expect("renders");
        assert!(
            text.contains("tiraka"),
            "the regular plural is -ka:\n{text}"
        );
        assert!(text.contains("tanka"), "{text}");
        assert!(
            text.contains("1 allomorph") && !text.contains("irregular"),
            "before any sound change the paradigm is regular:\n{text}"
        );
    }

    #[test]
    fn an_evolved_paradigm_renders_irregular_and_names_the_conditioning_rule() {
        let mut genome = proto();
        // Stand in for a real intervocalic-voicing run: voice the suffix /k/ of the
        // vowel-final stem's plural (tira-PL, flat index 4) and leave the
        // consonant-final stem's plural (tan-PL) untouched. Record the rule so the
        // renderer can name it, exactly as `evolve` would.
        genome.applied_rules = vec![stem_soundchange::SoundChangeRule {
            id: RuleId::new("r_ivv"),
            name: "Intervocalic voicing".to_owned(),
            description: String::new(),
            chronology_years: 250,
            target: stem_soundchange::SegmentPattern {
                features: stem_phonology::FeatureBundle::EMPTY,
                stress: None,
            },
            environment: stem_soundchange::Environment::default(),
            change: stem_soundchange::Change::Delete,
        }];

        // Rebuild the lexicon with a trace on tira-PL.
        let mut entries: Vec<WordEntry> = genome.lexicon.iter().cloned().collect();
        for entry in &mut entries {
            let is_tira_pl = entry
                .glosses
                .first()
                .map(|g| g == "star PL")
                .unwrap_or(false);
            if is_tira_pl {
                let input = entry.phonemic_form.clone();
                let trace = Derivation {
                    input: input.clone(),
                    steps: vec![stem_lexicon::RuleApplication {
                        rule: RuleId::new("r_ivv"),
                        index: 0,
                        sites: vec![stem_lexicon::SiteTrace {
                            at: 4,
                            before: PhonemeId::new("ph_k"),
                            after: Some(PhonemeId::new("ph_g")),
                            resolution: None,
                            left: vec![Some(PhonemeId::new("ph_a"))],
                            right: vec![Some(PhonemeId::new("ph_a"))],
                            emptied_syllable: None,
                        }],
                        blocked: vec![],
                    }],
                };
                entry.phonemic_form = trace.final_form();
                entry.trace = Some(trace);
            }
        }
        genome.lexicon = stem_lexicon::Lexicon::from_entries(entries);

        let paradigm = genome.morphology.paradigms[0].clone();
        let text = render_paradigm(&genome, &paradigm).expect("renders");
        assert!(
            text.contains("tira\u{0261}a"),
            "tira-PL voiced to tiraɡa:\n{text}"
        );
        assert!(text.contains("tanka"), "tan-PL stayed -ka:\n{text}");
        assert!(text.contains("2 allomorphs (irregular)"), "{text}");
        assert!(
            text.contains("Intervocalic voicing fired"),
            "the renderer names the conditioning rule:\n{text}"
        );
        assert!(
            text.contains("did not apply here"),
            "and marks the cell the rule skipped:\n{text}"
        );
    }

    #[test]
    fn render_paradigm_is_a_pure_function() {
        let genome = proto();
        let paradigm = genome.morphology.paradigms[0].clone();
        assert_eq!(
            render_paradigm(&genome, &paradigm).unwrap(),
            render_paradigm(&genome, &paradigm).unwrap()
        );
    }

    #[test]
    fn render_paradigm_errors_when_the_paradigm_is_not_inflected() {
        // A genome with the morphology but no inflected cells in the lexicon.
        let genome = LanguageGenome::proto("proto_x", "Proto-X")
            .with_phonemes(inventory())
            .with_morphology(morphology());
        let paradigm = genome.morphology.paradigms[0].clone();
        assert!(
            render_paradigm(&genome, &paradigm).is_err(),
            "an un-inflected paradigm cannot be rendered"
        );
    }

    /// Two cells that share an affix *set* but differ in affix *order* materialise
    /// different forms (`i·a·tira` vs `a·i·tira`), and the ordered join must keep
    /// each column on its own form — the sorted-multiset join it replaced would
    /// resolve both columns to the first-matching entry and print one form twice.
    #[test]
    fn cells_differing_only_in_affix_order_do_not_collide() {
        let prefix = |id: &str, seg: &str| Morpheme {
            id: MorphemeId::new(id),
            role: MorphemeRole::Prefix,
            gloss: id.to_owned(),
            form: stem_phonology::Root {
                syllables: vec![syllable("V", &[seg])],
            },
            part_of_speech: stem_lexicon::PartOfSpeech::Noun,
        };
        let morphology = Morphology {
            morphemes: vec![
                Morpheme {
                    id: MorphemeId::new("m_tira"),
                    role: MorphemeRole::Stem,
                    gloss: "star".to_owned(),
                    form: stem_phonology::Root {
                        syllables: vec![
                            syllable("CV", &["ph_t", "ph_i"]),
                            syllable("CV", &["ph_r", "ph_a"]),
                        ],
                    },
                    part_of_speech: stem_lexicon::PartOfSpeech::Noun,
                },
                prefix("m_x", "ph_i"),
                prefix("m_y", "ph_a"),
            ],
            paradigms: vec![Paradigm {
                id: "ORDER".to_owned(),
                name: "Order".to_owned(),
                stems: vec![MorphemeId::new("m_tira")],
                cells: vec![
                    ParadigmCell {
                        label: "A".to_owned(),
                        affixes: vec![MorphemeId::new("m_x"), MorphemeId::new("m_y")],
                    },
                    ParadigmCell {
                        label: "B".to_owned(),
                        affixes: vec![MorphemeId::new("m_y"), MorphemeId::new("m_x")],
                    },
                ],
            }],
        };
        let cells = stem_lexicon::inflect(
            &morphology.paradigms[0],
            &morphology.morphemes,
            &stem_core::LanguageId::new("proto_x"),
        )
        .expect("inflects");
        let genome = LanguageGenome::proto("proto_x", "Proto-X")
            .with_phonemes(inventory())
            .with_morphology(morphology)
            .with_lexicon(stem_lexicon::Lexicon::from_entries(cells));
        let paradigm = genome.morphology.paradigms[0].clone();

        let text = render_paradigm(&genome, &paradigm).expect("renders");
        assert!(
            text.contains("iatira"),
            "cell A (m_x·m_y·stem) shows i·a·tira:\n{text}"
        );
        assert!(
            text.contains("aitira"),
            "cell B (m_y·m_x·stem) shows a·i·tira, not A's form:\n{text}"
        );
    }
}
