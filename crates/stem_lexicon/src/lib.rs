//! Lexicon: words, their forms, and their ancestry.
//!
//! - [`concept`] — the built-in comparison list: what two languages are compared *at*.
//! - [`word`] — one entry in one language.
//! - [`lexicon`] — the ordered collection, and its validation.
//! - [`build`] — seeding a proto-lexicon from the concept list.
//!
//! # The invariant that matters
//!
//! > **For any two entries `a` and `b` in any two lexicons of one family,
//! > `a.cognate_set == b.cognate_set` if and only if `a` and `b` descend by
//! > unbroken inheritance from one and the same entry of that family's
//! > proto-lexicon.**
//!
//! This is the thread that ties `*takala` to Coastal `taal` and Highland `tazal`;
//! break it and the whole comparative view collapses (`DESIGN.md` §8.6,
//! `CLAUDE.md`). Three corollaries, all checkable today:
//!
//! 1. A [`stem_core::CognateSetId`] is **never a function of the word's form.**
//!    Forms change under every rule; a hashed form would reassign cognacy at each
//!    one — and `*takala` → `taal` / `tazal` must stay one set *precisely because*
//!    the forms diverged.
//! 2. It is **never minted from a bare per-language counter.** A daughter's counter
//!    restarts at 1 and would reuse its parent's strings for different words.
//! 3. It is **never derived from the concept.** Two synonyms in one language share
//!    a concept and must be different sets; and after M9's drift, a set named
//!    `cog_x_NOSE` whose reflex means "beak" is actively false.
//!
//! **What M4's fork must therefore do is copy `cognate_set` verbatim and never
//! mint.** That is the entire M4 obligation and it is one line of the fork.
//! [`build::scoped_cognate_set`] is the only minting site in the workspace.
//!
//! # Concept and cognate set are different identities
//!
//! At M2 they are in exact bijection and a future reader will want to delete one.
//! They must not. They diverge the moment M4 lands — a daughter's concept may drift
//! under M9 while its cognate set cannot change — and the moment synonyms land, two
//! entries sharing one concept with two cognate sets. Latin *caput* "head" and
//! French *chef* "chief" share a cognate set and not a concept.

pub mod build;
pub mod concept;
pub mod lexicon;
pub mod word;

pub use build::{build_proto_lexicon, scoped_cognate_set};
pub use concept::{CONCEPT_COUNT, CONCEPTS, Concept, ConceptKey, PartOfSpeech, SWADESH_COUNT};
pub use lexicon::{Lexicon, check_against_inventory, is_portable_id};
pub use word::{WordEntry, WordSource};

#[cfg(test)]
mod tests {
    use super::*;
    use stem_core::{CognateSetId, LanguageId, RngDomain, Validate, WordId, rng_for};
    use stem_phonology::{
        Phoneme, PhonemeInventory, Phonotactics, Root, RootGenerator, SegmentKind, Syllable,
        WeightedSyllableCount, WeightedTemplate,
    };

    fn inventory() -> PhonemeInventory {
        PhonemeInventory::from_phonemes([
            Phoneme::new("ph_t", "t", SegmentKind::Consonant).with_weight(50),
            Phoneme::new("ph_k", "k", SegmentKind::Consonant).with_weight(45),
            Phoneme::new("ph_m", "m", SegmentKind::Consonant).with_weight(35),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel).with_weight(60),
            Phoneme::new("ph_i", "i", SegmentKind::Vowel).with_weight(40),
        ])
    }

    fn phonotactics() -> Phonotactics {
        Phonotactics {
            templates: vec![
                WeightedTemplate::new("CV").with_weight(45),
                WeightedTemplate::new("CVC").with_weight(35),
            ],
            syllables_per_root: vec![
                WeightedSyllableCount::new(1).with_weight(25),
                WeightedSyllableCount::new(2).with_weight(55),
            ],
        }
    }

    fn build(count: usize, seed: u64) -> Lexicon {
        build_proto_lexicon(
            &LanguageId::new("proto_asterian"),
            &inventory(),
            &phonotactics(),
            &CONCEPTS[..count],
            seed,
        )
        .expect("the test language generates")
    }

    // --- ROADMAP M2's acceptance criteria ---

    #[test]
    fn every_generated_entry_has_a_stable_word_id_and_cognate_set_id() {
        let lexicon = build(CONCEPT_COUNT, 42);
        assert_eq!(lexicon.len(), CONCEPT_COUNT);
        for (i, entry) in lexicon.iter().enumerate() {
            assert_eq!(entry.id, WordId::sequential(i + 1));
            assert_eq!(
                entry.cognate_set,
                CognateSetId::new(format!("cog_proto_asterian_{:04}", i + 1))
            );
            assert!(!entry.id.is_empty() && !entry.cognate_set.is_empty());
        }
    }

    #[test]
    fn building_a_lexicon_twice_from_one_seed_produces_identical_entries() {
        assert_eq!(build(CONCEPT_COUNT, 42), build(CONCEPT_COUNT, 42));
    }

    #[test]
    fn a_different_seed_produces_a_different_lexicon() {
        assert_ne!(build(CONCEPT_COUNT, 1), build(CONCEPT_COUNT, 2));
    }

    /// The draw contract: nothing before entry N depends on how many come after.
    #[test]
    fn building_over_fewer_concepts_is_a_prefix_of_building_over_more() {
        let few = build(25, 7);
        let many = build(CONCEPT_COUNT, 7);
        assert_eq!(few.len(), 25);
        for (a, b) in few.iter().zip(many.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn every_generated_entry_has_a_distinct_cognate_set() {
        let lexicon = build(CONCEPT_COUNT, 42);
        let sets: std::collections::BTreeSet<&str> =
            lexicon.iter().map(|e| e.cognate_set.as_str()).collect();
        assert_eq!(sets.len(), lexicon.len());
    }

    #[test]
    fn every_generated_entry_names_its_concept_and_takes_its_part_of_speech() {
        let lexicon = build(CONCEPT_COUNT, 42);
        for (entry, concept) in lexicon.iter().zip(CONCEPTS) {
            assert_eq!(entry.concept.as_ref().unwrap().as_str(), concept.key);
            assert_eq!(entry.part_of_speech, concept.part_of_speech);
            assert_eq!(entry.display_gloss(), Some(concept.gloss));
            assert!(entry.glosses.is_empty(), "the concept supplies the gloss");
        }
    }

    #[test]
    fn a_generated_lexicon_validates_cleanly() {
        let lexicon = build(CONCEPT_COUNT, 42);
        let report = lexicon.validate();
        assert!(report.is_ok(), "{report}");
        let cross = check_against_inventory(&lexicon, &inventory());
        assert!(cross.is_ok(), "{cross}");
    }

    #[test]
    fn every_generated_segment_comes_from_the_inventory() {
        let inventory = inventory();
        for entry in build(CONCEPT_COUNT, 3).iter() {
            for id in entry.segments() {
                assert!(inventory.get(id).is_some(), "{id} escaped the inventory");
            }
        }
    }

    // --- The cognate-set invariant ---

    /// Proves the mint rule is fork-independent **without building forking**: two
    /// hand-built daughters that copied their parent's ids agree everywhere.
    #[test]
    fn two_hand_built_daughters_of_one_proto_share_every_cognate_set_id() {
        let proto = build(20, 42);

        // What M4's fork must do: copy `cognate_set` verbatim, change the forms.
        let daughter = |suffix: &str| -> Lexicon {
            Lexicon::from_entries(proto.iter().map(|e| {
                let mut copy = e.clone();
                copy.glosses = vec![format!("{} ({suffix})", e.display_gloss().unwrap())];
                copy
            }))
        };

        let coastal = daughter("coastal");
        let highland = daughter("highland");

        for entry in proto.iter() {
            let in_coastal = coastal.by_cognate_set(&entry.cognate_set);
            let in_highland = highland.by_cognate_set(&entry.cognate_set);
            assert!(
                in_coastal.is_some() && in_highland.is_some(),
                "cognate set {} did not survive the fork",
                entry.cognate_set
            );
        }
    }

    /// They are in bijection at M2 and a future reader will want to collapse them.
    /// Latin *caput* "head" → French *chef* "chief": same cognate set, different
    /// concept. The types must already allow it.
    #[test]
    fn a_cognate_set_and_a_concept_are_independent_identities() {
        let entry = WordEntry {
            id: WordId::new("w_0001"),
            concept: Some(ConceptKey::new("STAR")),
            phonemic_form: Root {
                syllables: vec![Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![
                        stem_core::PhonemeId::new("ph_t"),
                        stem_core::PhonemeId::new("ph_a"),
                    ],
            stress: None,
                }],
            },
            glosses: Vec::new(),
            part_of_speech: PartOfSpeech::Noun,
            // Deliberately NOT STAR's ordinal.
            cognate_set: CognateSetId::new("cog_proto_asterian_0099"),
            source: WordSource::Authored,
        };
        let text = ron::ser::to_string(&entry).expect("serialise");
        let back: WordEntry = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, entry);
        assert_eq!(back.concept.unwrap().as_str(), "STAR");
        assert_eq!(back.cognate_set.as_str(), "cog_proto_asterian_0099");
    }

    /// The id must decode even when the language id itself contains underscores
    /// and digits — split at the LAST underscore.
    #[test]
    fn a_cognate_set_id_decodes_uniquely_to_its_language_and_ordinal() {
        for language in ["proto_asterian", "a", "a_0001", "x-9_z", "0"] {
            let id = scoped_cognate_set(&LanguageId::new(language), 2);
            let text = id.as_str();
            let (head, ordinal) = text.rsplit_once('_').expect("an ordinal is present");
            assert_eq!(ordinal, "0002");
            assert_eq!(head.strip_prefix("cog_").expect("the prefix"), language);
        }
    }

    /// The invariant is a naming rule, so it is defended by reading the sources
    /// rather than by runtime indirection. Crude, and honest about being so.
    #[test]
    fn only_proto_lexicon_construction_mints_a_cognate_set() {
        let sources = [
            ("build.rs", include_str!("build.rs")),
            ("lexicon.rs", include_str!("lexicon.rs")),
            ("word.rs", include_str!("word.rs")),
            ("concept.rs", include_str!("concept.rs")),
        ];
        for (name, src) in sources {
            for (n, line) in src.lines().enumerate() {
                let is_call = line.contains("CognateSetId::new")
                    && !line.trim_start().starts_with("//")
                    && !line.trim_start().starts_with("///");
                if is_call {
                    assert_eq!(
                        name,
                        "build.rs",
                        "{name}:{} mints a CognateSetId; only `scoped_cognate_set` may",
                        n + 1
                    );
                }
            }
        }
    }

    // --- Validation ---

    fn one_entry(concept: Option<&str>, glosses: Vec<String>) -> Lexicon {
        Lexicon::from_entries([WordEntry {
            id: WordId::new("w_0001"),
            concept: concept.map(ConceptKey::new),
            phonemic_form: Root {
                syllables: vec![Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![
                        stem_core::PhonemeId::new("ph_t"),
                        stem_core::PhonemeId::new("ph_a"),
                    ],
            stress: None,
                }],
            },
            glosses,
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: CognateSetId::new("cog_x_0001"),
            source: WordSource::Authored,
        }])
    }

    #[test]
    fn a_misspelled_concept_key_warns_and_suggests_the_nearest_real_one() {
        let report = one_entry(Some("NOES"), vec!["nose".to_owned()]).validate();
        assert!(report.is_ok(), "unusual is not broken: {report}");
        let issue = report
            .warnings()
            .find(|i| i.code == "unknown_concept")
            .unwrap_or_else(|| panic!("{report}"));
        assert!(issue.message.contains("NOSE"), "{}", issue.message);
    }

    #[test]
    fn a_word_with_an_unknown_concept_but_its_own_gloss_stays_valid() {
        let report = one_entry(Some("OBSIDIAN"), vec!["black glass".to_owned()]).validate();
        assert!(report.is_ok(), "{report}");
        assert!(report.warnings().any(|i| i.code == "unknown_concept"));
    }

    #[test]
    fn a_word_with_no_resolvable_concept_and_no_gloss_is_an_error() {
        let report = one_entry(Some("OBSIDIAN"), Vec::new()).validate();
        assert!(report.errors().any(|i| i.code == "no_gloss"), "{report}");
    }

    #[test]
    fn a_word_with_neither_concept_nor_gloss_is_an_error() {
        let report = one_entry(None, Vec::new()).validate();
        assert!(report.errors().any(|i| i.code == "no_gloss"), "{report}");
    }

    #[test]
    fn duplicate_word_ids_are_an_error_and_duplicate_cognate_sets_only_warn() {
        let mut lexicon = build(2, 42);
        let mut clash = lexicon.iter().next().unwrap().clone();
        clash.concept = Some(ConceptKey::new("STAR"));
        lexicon.push(clash);

        let report = lexicon.validate();
        assert!(
            report.errors().any(|i| i.code == "duplicate_word_id"),
            "{report}"
        );
        assert!(
            report.warnings().any(|i| i.code == "duplicate_cognate_set"),
            "doublets are legitimate, so this must be a warning: {report}"
        );
    }

    #[test]
    fn an_empty_lexicon_notes_rather_than_erroring() {
        let report = Lexicon::new().validate();
        assert!(report.is_ok());
        assert!(report.issues.iter().any(|i| i.code == "empty"));
    }

    #[test]
    fn a_segment_outside_the_inventory_is_an_error() {
        let mut lexicon = one_entry(Some("STAR"), vec![]);
        // `ph_zzz` is in no inventory anywhere.
        lexicon.push(WordEntry {
            id: WordId::new("w_0002"),
            concept: Some(ConceptKey::new("SUN")),
            phonemic_form: Root {
                syllables: vec![Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![
                        stem_core::PhonemeId::new("ph_zzz"),
                        stem_core::PhonemeId::new("ph_a"),
                    ],
            stress: None,
                }],
            },
            glosses: Vec::new(),
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: CognateSetId::new("cog_x_0002"),
            source: WordSource::Authored,
        });

        let report = check_against_inventory(&lexicon, &inventory());
        assert!(
            report.errors().any(|i| i.code == "unknown_phoneme"),
            "{report}"
        );
    }

    #[test]
    fn a_syllable_whose_pattern_disagrees_with_its_segments_warns_but_stays_valid() {
        let lexicon = Lexicon::from_entries([WordEntry {
            id: WordId::new("w_0001"),
            concept: Some(ConceptKey::new("STAR")),
            phonemic_form: Root {
                syllables: vec![Syllable {
                    // Says CVC, holds CV — what M3's vowel loss will produce.
                    pattern: "CVC".to_owned(),
                    segments: vec![
                        stem_core::PhonemeId::new("ph_t"),
                        stem_core::PhonemeId::new("ph_a"),
                    ],
            stress: None,
                }],
            },
            glosses: Vec::new(),
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: CognateSetId::new("cog_x_0001"),
            source: WordSource::Authored,
        }]);
        let report = check_against_inventory(&lexicon, &inventory());
        assert!(
            report.is_ok(),
            "nothing reads pattern, so nothing is broken: {report}"
        );
        assert!(
            report
                .warnings()
                .any(|i| i.code == "syllable_shape_mismatch"),
            "{report}"
        );
    }

    /// Two phonemes may share a romanisation, so `Root` equality is not the same
    /// question as "does the dictionary print this headword twice?".
    #[test]
    fn homophones_are_detected_over_the_written_form_not_over_segment_ids() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_k", "k", SegmentKind::Consonant),
            // Distinct phoneme, distinct IPA, same romanisation.
            Phoneme::new("ph_q", "q", SegmentKind::Consonant).with_romanization("k"),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
        ]);
        let word = |id: &str, first: &str, set: &str| WordEntry {
            id: WordId::new(id),
            concept: None,
            phonemic_form: Root {
                syllables: vec![Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![
                        stem_core::PhonemeId::new(first),
                        stem_core::PhonemeId::new("ph_a"),
                    ],
            stress: None,
                }],
            },
            glosses: vec!["thing".to_owned()],
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: CognateSetId::new(set),
            source: WordSource::Authored,
        };
        let lexicon = Lexicon::from_entries([
            word("w_0001", "ph_k", "cog_x_0001"),
            word("w_0002", "ph_q", "cog_x_0002"),
        ]);

        // The two Roots differ; the rendered forms do not.
        assert_ne!(
            lexicon.iter().next().unwrap().phonemic_form,
            lexicon.iter().nth(1).unwrap().phonemic_form
        );
        let report = check_against_inventory(&lexicon, &inventory);
        assert!(
            report.issues.iter().any(|i| i.code == "homophones"),
            "both print `ka`, so the reader sees a repeat: {report}"
        );
    }

    // --- The validator and the engine must not disagree ---

    /// Direction one: anything `generate-roots` accepts, `new-lexicon` must accept.
    #[test]
    fn a_language_that_can_generate_roots_can_seed_a_lexicon() {
        let inventory = inventory();
        let phonotactics = phonotactics();
        assert!(stem_phonology::RootGenerator::new(&inventory, &phonotactics).is_ok());
        assert!(
            build_proto_lexicon(
                &LanguageId::new("x"),
                &inventory,
                &phonotactics,
                &CONCEPTS[..5],
                1
            )
            .is_ok()
        );
    }

    /// The lexicon twin of M1's `a_feature_fault_does_not_block_generation`.
    #[test]
    fn a_feature_fault_does_not_block_lexicon_seeding() {
        let mut inventory = inventory();
        inventory.push(Phoneme::new("ph_e", "e", SegmentKind::Vowel).with_features(
            stem_phonology::FeatureBundle::EMPTY.with(
                stem_phonology::Feature::Syllabic,
                stem_phonology::Sign::Plus,
            ),
        ));
        assert!(
            inventory
                .validate()
                .errors()
                .any(|i| i.code == "missing_required_feature"),
            "the inventory really is feature-broken"
        );
        assert!(
            build_proto_lexicon(
                &LanguageId::new("x"),
                &inventory,
                &phonotactics(),
                &CONCEPTS[..5],
                1
            )
            .is_ok(),
            "a feature fault must not block a lexicon the generator would produce"
        );
    }

    // --- Round-trip and properties ---

    #[test]
    fn a_lexicon_round_trips_through_ron_unchanged() {
        let original = build(10, 42);
        let text = ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default())
            .expect("serialise");
        let back: Lexicon = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, original);
    }

    #[test]
    fn a_misspelled_word_field_fails_to_load_rather_than_defaulting() {
        let text = r#"[(
            id: "w_0001",
            concept: "STAR",
            phonemic_form: (syllables: [(pattern: "CV", segments: ["ph_t", "ph_a"])]),
            part_of_speech: noun,
            cognate_st: "cog_x_0001",
        )]"#;
        assert!(
            ron::from_str::<Lexicon>(text).is_err(),
            "a misspelled field must not silently sever a word from its family"
        );
    }

    /// Property (`DESIGN.md` §16.3) over many streams.
    #[test]
    fn generated_lexicons_always_have_unique_ids_and_inventory_segments() {
        let inventory = inventory();
        let phonotactics = phonotactics();
        for seed in 0..500u64 {
            let lexicon = build_proto_lexicon(
                &LanguageId::new("x"),
                &inventory,
                &phonotactics,
                &CONCEPTS[..20],
                seed,
            )
            .expect("generates");

            let ids: std::collections::BTreeSet<&str> =
                lexicon.iter().map(|e| e.id.as_str()).collect();
            let sets: std::collections::BTreeSet<&str> =
                lexicon.iter().map(|e| e.cognate_set.as_str()).collect();
            assert_eq!(ids.len(), lexicon.len(), "seed {seed}: duplicate word id");
            assert_eq!(
                sets.len(),
                lexicon.len(),
                "seed {seed}: duplicate cognate set"
            );

            for entry in lexicon.iter() {
                assert!(!entry.phonemic_form.is_empty());
                for id in entry.segments() {
                    assert!(inventory.get(id).is_some(), "seed {seed}: {id} escaped");
                }
            }
        }
    }

    /// The lexicon stream must be its own, or an M2 change would silently rewrite
    /// M1 output.
    #[test]
    fn the_lexicon_stream_is_independent_of_the_roots_stream() {
        let mut roots_rng = rng_for(42, RngDomain::Roots);
        let mut lexicon_rng = rng_for(42, RngDomain::Lexicon);
        let inventory = inventory();
        let phonotactics = phonotactics();
        let generator = RootGenerator::new(&inventory, &phonotactics).unwrap();
        assert_ne!(
            generator.next_root(&mut roots_rng),
            generator.next_root(&mut lexicon_rng)
        );
    }
}
