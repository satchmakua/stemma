//! The ROADMAP M22 acceptance tests: **a proposed rule set is applied through the
//! ordinary `apply-rules` path and produces a traced, reproducible result; a proposal
//! that fails validation is refused and reported, never partially applied; with the
//! assistant disabled every other command behaves identically, byte for byte.**
//!
//! # The clause that carries the milestone
//!
//! *No LLM output may mutate a language without passing through validation* (§3.2) —
//! enforced since M0, when there was nothing to enforce it against.
//!
//! The way that constraint dies is not by someone writing a bypass. It dies by someone
//! adding a *second* path: a fast route for machine-written artefacts, a field that
//! marks a rule set as proposed, a validator tuned for what models tend to get wrong.
//! Each of those is reasonable on its own and each ends with two code paths, one of
//! which is less examined than the other.
//!
//! So the load-bearing test here is not the refusal — it is
//! `an_accepted_proposal_is_byte_identical_to_the_same_rules_typed_by_hand`. The same
//! `RuleSet`, applied through `stem_assist::accept` and through `LanguageGenome::evolve`,
//! must produce languages that serialise to the same bytes. If a difference ever
//! appears there, a second path exists, whatever the documentation says.
//!
//! # The two fixtures
//!
//! Both were written by a model (Claude Opus 5) from `stemma brief`, and say so.
//!
//! - `proposal_raising.ron` is accepted: a merger and a split, two ordinary changes.
//! - `proposal_incoherent.ron` is refused, and refused for the *interesting* reason.
//!   Its prose is correct linguistics — nasal place assimilation, §7.2's own example —
//!   and its rule copies from a slot the environment never declared. Reading the
//!   rationale finds nothing wrong. Only running it does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_assist::{Artefact, Proposal, ProposalKind, Provenance};
use stem_genome::LanguageGenome;

const STEMMA: &str = env!("CARGO_BIN_EXE_stemma");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

/// Proto-Asterian with its vocabulary coined — the language every proposal targets.
fn proto() -> LanguageGenome {
    let genome: LanguageGenome = stem_io::load(fixture("proto_asterian.ron")).expect("loads");
    let lexicon = stem_lexicon::build_shaped_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        &stem_lexicon::meanings(&genome.concepts),
        &genome.environment,
        genome.seed,
    )
    .expect("coins");
    genome.with_lexicon(lexicon)
}

fn proposal(name: &str) -> Proposal {
    stem_io::load(fixture(name)).expect("the proposal loads")
}

/// The accepted one, on disk, so the CLI can be pointed at a language with a lexicon.
fn proto_on_disk() -> String {
    let path = std::env::temp_dir().join("stemma_m22_proto.ron");
    stem_io::save(&path, &proto()).expect("saves");
    path.display().to_string()
}

// -------------------------------------------------- half one: it goes through

/// **ROADMAP M22's acceptance, first clause.** A proposal is applied by the ordinary
/// path and the result is traced and reproducible.
#[test]
fn an_accepted_proposal_produces_a_traced_reproducible_language() {
    let genome = proto();
    let proposal = proposal("proposal_raising.ron");

    let (daughter, _) =
        stem_assist::accept(&proposal, &genome, "raised", "Raised Asterian", 450).expect("accepts");

    // Traced: every word carries the derivation that produced it, and it replays.
    let mut checked = 0usize;
    for entry in daughter.lexicon.iter() {
        let Some(trace) = &entry.trace else { continue };
        // `replay` returns the form after each recorded step. `WordEntry::trace` has
        // three states and this is two of them: a word some rule changed ends at its
        // stored form, and a word exposed to every rule and changed by none has no
        // steps at all and still stores its input. Collapsing the two would let a
        // silently-dropped step pass.
        let forms = trace.replay();
        match forms.last() {
            Some(last) => {
                assert_eq!(
                    last, &entry.phonemic_form,
                    "`{}`'s recorded derivation must rebuild its stored form",
                    entry.id
                );
                checked += 1;
            }
            None => assert_eq!(
                trace.input, entry.phonemic_form,
                "`{}` was changed by no rule, so it must still be its input",
                entry.id
            ),
        }
    }
    assert!(checked > 0, "the proposal actually changed words");

    // Reproducible: the same proposal against the same language, twice.
    let (again, _) =
        stem_assist::accept(&proposal, &genome, "raised", "Raised Asterian", 450).expect("accepts");
    assert_eq!(daughter, again, "two runs, byte for byte");

    // And it is an ordinary daughter: parent edge, rule history, the lot.
    assert_eq!(daughter.parent.as_ref(), Some(&genome.id));
    assert_eq!(daughter.applied_rules.len(), 2);
}

/// **The load-bearing test.** The same `RuleSet` through the proposal path and through
/// `evolve` produces the *same bytes*.
///
/// This is what "no second code path" means operationally. A field marking a rule set
/// as machine-proposed, a validator tuned for model mistakes, a fast route for
/// artefacts that came from a briefing — each is reasonable alone, each ends with two
/// paths, and each would show up here as a difference.
#[test]
fn an_accepted_proposal_is_byte_identical_to_the_same_rules_typed_by_hand() {
    let genome = proto();
    let proposal = proposal("proposal_raising.ron");
    let Artefact::Rules(rules) = &proposal.artefact else {
        panic!("the fixture proposes rules");
    };

    let (through_assistant, _) =
        stem_assist::accept(&proposal, &genome, "d", "Daughter", 450).expect("accepts");
    let (by_hand, _) = genome.evolve("d", "Daughter", rules, 450).expect("evolves");

    assert_eq!(
        through_assistant, by_hand,
        "the engine must not be able to tell who wrote the rules"
    );

    // And the same through the file format, since that is what a user compares.
    let a = std::env::temp_dir().join("stemma_m22_assistant.ron");
    let b = std::env::temp_dir().join("stemma_m22_byhand.ron");
    stem_io::save(&a, &through_assistant).expect("saves");
    stem_io::save(&b, &by_hand).expect("saves");
    assert_eq!(
        std::fs::read(&a).expect("read"),
        std::fs::read(&b).expect("read"),
        "byte for byte on disk, too"
    );
}

/// **The model's prose never reaches the language.** It is printed, fenced and
/// attributed — and it appears nowhere in the file the acceptance produces.
///
/// Without the envelope a rationale ends up in a `note:` inside the rule set, stored in
/// the language, indistinguishable from the author's own words. The fixture's rationale
/// is long and distinctive, so a leak of any part of it is detectable.
#[test]
fn the_proposers_prose_is_labelled_as_prose_and_never_stored() {
    let proposal = proposal("proposal_raising.ron");
    assert!(
        proposal
            .rationale
            .contains("phonemic merger and phonemic split"),
        "the fixture's rationale is the distinctive string this test hunts for"
    );

    // Printed, fenced, and attributed to the proposer rather than the engine.
    let out = stdout(&stemma(&[
        "review",
        &proto_on_disk(),
        "--proposal",
        fixture("proposal_raising.ron").to_str().expect("path"),
    ]));
    assert!(
        out.contains("the proposer's own words, not the engine's"),
        "prose must be labelled as prose: {out}"
    );
    assert!(
        out.contains("nothing above was parsed, matched, or stored"),
        "{out}"
    );
    assert!(out.contains("phonemic merger and phonemic split"), "{out}");

    // And absent from the language it produces.
    let genome = proto();
    let (daughter, _) =
        stem_assist::accept(&proposal, &genome, "raised", "Raised", 450).expect("accepts");
    let saved = std::env::temp_dir().join("stemma_m22_prose.ron");
    stem_io::save(&saved, &daughter).expect("saves");
    let text = std::fs::read_to_string(&saved).expect("readable");

    for leak in [
        "phonemic merger and phonemic split",
        "I want to be plain",
        "least sure of",
        "claude-opus-5",
    ] {
        assert!(
            !text.contains(leak),
            "`{leak}` reached the language file; the rationale must stay in the \
             proposal, which is the only thing keeping it distinguishable from the \
             author's own notes"
        );
    }
}

// ------------------------------------------------------- half two: it refuses

/// **ROADMAP M22's acceptance, second clause.** A proposal that fails validation is
/// refused and reported — and refused *whole*.
#[test]
fn a_proposal_that_fails_validation_is_refused_and_nothing_is_applied() {
    let genome = proto();
    let proposal = proposal("proposal_incoherent.ron");

    let verdict = stem_assist::review(&proposal, &genome).expect("reviews");
    assert!(!verdict.accepted, "{}", verdict.summary);
    assert!(
        verdict
            .report
            .errors()
            .any(|i| i.code == "rules.change_references_unmatched_position"),
        "the engine names the incoherence in its own words: {}",
        verdict.report
    );
    assert!(
        verdict.summary.contains("nothing was applied"),
        "{}",
        verdict.summary
    );

    // Not partially applied: `accept` errors, and the language it was given is
    // untouched — `evolve` never mutates its argument, so a refusal cannot leave half
    // a history behind.
    let before = genome.clone();
    let refused = stem_assist::accept(&proposal, &genome, "x", "X", 100);
    assert!(refused.is_err(), "a refused proposal produces no language");
    assert_eq!(genome, before, "and leaves the original exactly as it was");
}

/// The refusal is **whole**. The incoherent set's *second* rule is perfectly fine and
/// does not run either — a language carrying half a proposed history is one nobody can
/// reason about.
#[test]
fn a_refusal_takes_the_good_rules_down_with_the_bad_one() {
    let proposal = proposal("proposal_incoherent.ron");
    let Artefact::Rules(rules) = &proposal.artefact else {
        panic!("the fixture proposes rules");
    };
    assert_eq!(rules.rules.len(), 2, "one broken rule and one sound one");

    // The sound one, on its own, applies cleanly — so its failure below is the
    // refusal being whole rather than the rule being bad.
    let genome = proto();
    let mut alone = rules.clone();
    alone.rules.retain(|r| r.id.as_str() == "r_a02");
    assert!(
        genome.evolve("solo", "Solo", &alone, 500).is_ok(),
        "the second rule is fine by itself"
    );

    assert!(stem_assist::accept(&proposal, &genome, "x", "X", 500).is_err());
}

/// A verdict can never differ from what accepting would do, because `review` **is** the
/// accept call with the result discarded. A gate that could say "accepted" where accept
/// then refused would be theatre.
#[test]
fn a_review_never_disagrees_with_what_accepting_would_do() {
    let genome = proto();
    for name in ["proposal_raising.ron", "proposal_incoherent.ron"] {
        let proposal = proposal(name);
        let verdict = stem_assist::review(&proposal, &genome).expect("reviews");
        let accepted = stem_assist::accept(&proposal, &genome, "x", "X", 100).is_ok();
        assert_eq!(
            verdict.accepted, accepted,
            "`{name}`: the review said {} and accepting said {accepted}",
            verdict.accepted
        );
    }
}

/// The CLI's exit code follows the verdict, so a script can gate on it without parsing
/// prose.
#[test]
fn the_exit_code_says_whether_the_proposal_was_accepted() {
    let path = proto_on_disk();
    let good = stemma(&[
        "review",
        &path,
        "--proposal",
        fixture("proposal_raising.ron").to_str().expect("path"),
    ]);
    assert!(good.status.success(), "{}", stdout(&good));

    let bad = stemma(&[
        "review",
        &path,
        "--proposal",
        fixture("proposal_incoherent.ron").to_str().expect("path"),
    ]);
    assert!(!bad.status.success(), "a refusal exits non-zero");
    assert!(stdout(&bad).contains("refused"), "{}", stdout(&bad));
}

/// `stemma accept` without `--out` writes nothing — M16's undo-is-the-file rule.
#[test]
fn accepting_without_an_output_path_writes_nothing() {
    let out = stdout(&stemma(&[
        "accept",
        &proto_on_disk(),
        "--proposal",
        fixture("proposal_raising.ron").to_str().expect("path"),
        "--id",
        "raised",
        "--name",
        "Raised Asterian",
        "--years",
        "450",
    ]));
    assert!(
        out.contains("(no --out given, so nothing was written)"),
        "{out}"
    );
}

/// A proposal written for a different language is refused before anything is tried. The
/// artefact may be perfectly valid and still be about somebody else's phonology.
#[test]
fn a_proposal_aimed_at_another_language_is_refused_outright() {
    let genome = proto();
    let mut wrong = proposal("proposal_raising.ron");
    wrong.target = "some_other_language".into();

    let err = stem_assist::review(&wrong, &genome).expect_err("refused");
    let message = err.to_string();
    assert!(
        message.contains("does not target this language"),
        "{message}"
    );
    assert!(
        stem_assist::accept(&wrong, &genome, "x", "X", 0).is_err(),
        "and accepting refuses on the same check"
    );
}

// ---------------------------------------------------- half three: the briefing

/// The briefing is generated from the language, so it cannot be out of date with
/// respect to the thing being proposed against — and it is deterministic.
#[test]
fn the_briefing_states_the_features_the_rules_must_be_written_over() {
    let genome = proto();
    let brief = stem_assist::render_briefing(&genome, ProposalKind::Rules).expect("renders");

    assert!(
        brief.contains("Rules match features, not letters"),
        "the constraint a model most needs told: {brief}"
    );
    // Every phoneme, with its real feature bundle — not a summary of one.
    for phoneme in genome.phonemes.iter() {
        assert!(
            brief.contains(phoneme.id.as_str()) && brief.contains(&phoneme.features.render()),
            "`{}`'s features must be in the briefing verbatim",
            phoneme.id
        );
    }
    // And what gets refused, so a proposer can avoid it rather than discover it.
    assert!(
        brief.contains("change_references_unmatched_position"),
        "{brief}"
    );
    assert!(
        brief.contains("Warnings are **not** refusals"),
        "the difference between a report and a refusal: {brief}"
    );

    assert_eq!(
        brief,
        stem_assist::render_briefing(&genome, ProposalKind::Rules).expect("renders"),
        "two runs, byte for byte"
    );
}

/// The briefing tells the proposer where prose is welcome, because otherwise it ends up
/// inside the artefact.
#[test]
fn the_briefing_says_where_reasoning_goes() {
    let brief = stem_assist::render_briefing(&proto(), ProposalKind::Rules).expect("renders");
    assert!(brief.contains("rationale"), "{brief}");
    assert!(brief.contains("the only place it is welcome"), "{brief}");
    assert!(
        brief.contains("no path from what you write to a stored form that skips"),
        "and states the constraint it is operating under: {brief}"
    );
}

// ------------------------------------------------------------- the invariants

/// **With the assistant disabled, every other command behaves identically.**
///
/// Operationally: the assistant added **no genome field**. A proposal is a separate
/// file, like a rule set — so every language written before M22 loads and saves with
/// exactly the bytes it had, and no command's output can have moved.
///
/// A `proposed_by` stamp on the genome was the tempting alternative and is refused on
/// purpose: an accepted rule set is the same artefact whichever hand wrote it, and a
/// field recording otherwise would be the engine learning a distinction §3.2 forbids
/// it to make.
#[test]
fn the_assistant_added_no_field_to_a_language() {
    for name in ["proto_asterian.ron", "asterian_attested.ron"] {
        let genome: LanguageGenome = stem_io::load(fixture(name)).expect("loads");
        let path = std::env::temp_dir().join(format!("stemma_m22_{name}"));
        stem_io::save(&path, &genome).expect("saves");
        let text = std::fs::read_to_string(&path).expect("readable");

        for token in [
            "proposal",
            "provenance",
            "rationale",
            "proposed_by",
            "assist",
        ] {
            assert!(
                !text.contains(token),
                "`{name}` gained a `{token}` field; the assistant must be invisible to \
                 a language that never met one"
            );
        }
        let back: LanguageGenome = stem_io::load(&path).expect("reloads");
        assert_eq!(back, genome);
    }
}

/// **`stem_assist` does no linguistics.** It is an envelope and a gate; every arm of
/// `accept` hands the artefact to a function an authored file would have reached.
///
/// A source scan, the shape of `stem_soundchange`'s three guards and the cognate-mint
/// scan. If the assistant ever builds a `WordEntry`, mints a cognate set, or touches a
/// `Derivation`, it has started doing the engine's job — and a second implementation of
/// the engine is exactly how the constraint dies quietly.
#[test]
fn the_assistant_never_does_the_engines_job() {
    let sources = [
        ("lib.rs", include_str!("../../stem_assist/src/lib.rs")),
        ("brief.rs", include_str!("../../stem_assist/src/brief.rs")),
        ("render.rs", include_str!("../../stem_assist/src/render.rs")),
    ];
    for (name, src) in sources {
        for (n, line) in src.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for banned in [
                "scoped_cognate_set",
                "WordEntry {",
                "Derivation",
                "RuleApplicationTrace",
                "apply_rules(",
                "apply_drift(",
                "next_root",
                "StemmaRng",
            ] {
                assert!(
                    !code.contains(banned),
                    "stem_assist/{name}:{} does the engine's job (`{banned}`); the \
                     assistant proposes and gates, and everything else must go through \
                     `evolve` / `drift` / `apply_edit` so there is one code path",
                    n + 1
                );
            }
        }
    }
}

/// A proposal round-trips through the file format, because a file a human can read
/// before applying is the whole safety story.
#[test]
fn a_proposal_round_trips_and_refuses_a_misspelled_field() {
    let original = proposal("proposal_raising.ron");
    let path = std::env::temp_dir().join("stemma_m22_roundtrip.ron");
    stem_io::save(&path, &original).expect("saves");
    let back: Proposal = stem_io::load(&path).expect("reloads");
    assert_eq!(back, original);

    // `deny_unknown_fields`: a typo fails to load rather than silently defaulting, so
    // a rationale misspelled as `rational` cannot be quietly dropped.
    // A non-empty rationale, because it is `skip_serializing_if` empty — a blank one
    // does not appear in the text, and misspelling a field that is not there proves
    // nothing.
    let bad = Proposal {
        id: "x".to_owned(),
        target: "proto_asterian".into(),
        provenance: Provenance::default(),
        rationale: "something to misspell the field of".to_owned(),
        artefact: original.artefact.clone(),
    };
    let text = ron::ser::to_string(&bad).expect("serialise");
    assert!(
        ron::from_str::<Proposal>(&text.replace("rationale", "rational")).is_err(),
        "a misspelled field must fail to load"
    );
}
