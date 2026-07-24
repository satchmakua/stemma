//! The `stemma` command-line interface.
//!
//! The CLI is the primary interface until M9 — `DESIGN.md` §20.5 is explicit that
//! the UI must not be started before the engine works. Every engine capability
//! should be reachable from here first; the eventual desktop app is a second front
//! end onto the same crates, never a place where logic lives.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use stem_core::{RngDomain, Severity, Validate, ValidationReport, rng_for};
use stem_genome::LanguageGenome;
use stem_lexicon::{CONCEPT_COUNT, CONCEPTS, build_proto_lexicon};
use stem_phonology::RootGenerator;

/// The most roots one invocation will generate.
///
/// Ten million is far past any real lexicon and comfortably inside what a `Vec`
/// can reserve. The bound exists so that a fat-fingered `--count` is a message
/// rather than a capacity-overflow panic or an OOM abort.
const MAX_ROOTS: u64 = 10_000_000;

/// Grow, evolve, fork, and trace fictional languages.
#[derive(Debug, Parser)]
#[command(name = "stemma", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check a language file for structural errors and typological oddities.
    Validate {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
    },

    /// Print the §17 plausibility profile: scored typological dimensions plus the
    /// graded report. Describes the language; it does not police it.
    Profile {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
    },

    /// Print a summary of a language.
    Info {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
    },

    /// Convert a language file between RON and JSON.
    Convert {
        /// The file to read.
        input: PathBuf,
        /// The file to write; its extension selects the output format.
        output: PathBuf,
    },

    /// Generate root words from the language's inventory and phonotactics.
    GenerateRoots {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
        /// How many roots to generate.
        ///
        /// Bounded: the result is collected into a `Vec`, which pre-reserves from
        /// the iterator's size hint, so an unbounded value aborts the process on
        /// allocation before drawing anything. A named error beats a capacity
        /// overflow panic.
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u64).range(1..=MAX_ROOTS))]
        count: u64,
        /// Override the language's own seed. Omitted, the file's seed is used, so
        /// a language always reproduces itself.
        #[arg(long)]
        seed: Option<u64>,
        /// Print IPA forms instead of the romanisation.
        #[arg(long)]
        ipa: bool,
    },

    /// Show each phoneme's resolved feature matrix.
    Features {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
        /// Show only the segment with this IPA form.
        #[arg(long)]
        ipa: Option<String>,
    },

    /// Seed a proto-lexicon: one word per concept on the built-in list.
    NewLexicon {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
        /// Override the language's own seed.
        #[arg(long)]
        seed: Option<u64>,
        /// How many concepts to coin words for, from the top of the list.
        #[arg(long, value_parser = clap::value_parser!(u16).range(1..=CONCEPT_COUNT as i64))]
        concepts: Option<u16>,
        /// Write the language back out with its new lexicon, instead of printing.
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Write the lexicon as a Markdown dictionary.
    ExportMd {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
        /// Write to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Write the lexicon as CLDF-shaped CSV.
    ExportCsv {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
        /// Write to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Apply an ordered rule set, producing the next stage of the lineage.
    ApplyRules {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
        /// Path to a rule-set file (`.ron` or `.json`).
        #[arg(long)]
        rules: PathBuf,
        /// The evolved language's id.
        #[arg(long)]
        id: String,
        /// The evolved language's display name.
        #[arg(long)]
        name: String,
        /// Simulated years the changes span. Absolute time runs from the lineage
        /// root, so this is added to the parent's depth; negatives are rejected —
        /// a stratum earlier than its parent would slip the genome gate
        /// (`negative_lineage_depth` needs *total* < 0), matching `fork`.
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(i32).range(0..))]
        years: i32,
        /// Write the evolved language here; omitted, print a summary only.
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Print one word's recorded derivation, rule by rule.
    Trace {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
        /// The word to trace, by id (`w_0001`).
        word: String,
    },

    /// Validate and summarise a rule-set file.
    Rules {
        /// Path to a rule-set file (`.ron` or `.json`).
        path: PathBuf,
    },

    /// Fork a language into a daughter — a verbatim copy under a new identity,
    /// or (with `--rules`) a daughter that has already undergone sound changes.
    ///
    /// Bare, this is `LanguageGenome::fork`: same phonology, lexicon, cognate
    /// sets, and history as the parent, with a parent edge back. With `--rules`
    /// it is exactly `apply-rules` under a verb that says "a sister branched
    /// off" rather than "the language advanced a stage" — the file records no
    /// verb, so the two are the same shape.
    Fork {
        /// Path to the parent language (`.ron` or `.json`).
        parent: PathBuf,
        /// The daughter's id.
        #[arg(long)]
        id: String,
        /// The daughter's display name.
        #[arg(long)]
        name: String,
        /// A rule set to apply as the daughter branches. Omitted, the daughter
        /// is a verbatim copy at the split instant.
        #[arg(long)]
        rules: Option<PathBuf>,
        /// Simulated years the split (and any changes) span. Absolute time is
        /// measured from the lineage root, so this is added to the parent's
        /// depth. Negatives are rejected — time runs forwards.
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(i32).range(0..))]
        years: i32,
        /// Write the daughter here; omitted, print its lexicon as a summary.
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Assemble a family from several language files and report on its lineage.
    ///
    /// Descent is read from each genome's `parent` field; nothing about the
    /// graph is stored. Prints the family tree and cognate coverage, then the
    /// family-level validation report.
    Family {
        /// The language files to assemble, in the order they should appear.
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    /// Print the §10.3 comparative table: reflexes of each meaning across a
    /// family, joined by cognate set (shared ancestry).
    ///
    /// Columns are the languages you pass, in order — the first is the reference
    /// the meanings resolve against, and its forms are the reconstructed
    /// proto-forms (marked `*`). A meaning is matched to a word by its displayed
    /// gloss, so `king` finds the word glossed "king".
    Cognates {
        /// The language files to compare, in column order (the first is the
        /// reference).
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
        /// The meanings to tabulate — one row each, in the given order.
        #[arg(long, required = true, num_args = 1..)]
        meanings: Vec<String>,
    },

    /// Trace one word by MEANING (§10.2): resolve the meaning to a word, then
    /// print its full derivation, rule by rule. The meaning-addressed sibling of
    /// `trace`.
    TraceWord {
        /// Path to a language file (`.ron` or `.json`).
        path: PathBuf,
        /// The meaning to trace, as an English gloss (`star`, `king`).
        meaning: String,
    },

    /// Produce the "Growing a Language Family in 90 Seconds" artefact (§21) as one
    /// self-contained Markdown document.
    ///
    /// The proto-language and its three rule histories are compiled into the
    /// binary, so the demo needs no fixtures on disk and runs identically from any
    /// directory. Deterministic: two runs are byte-identical.
    Demo {
        /// Write the document here; omitted, print to stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            // `{err:?}` on an anyhow error prints the whole context chain, which is
            // what makes "could not read X: no such file" readable rather than bare.
            eprintln!("error: {err:?}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate { path } => validate(&path),
        Command::Profile { path } => profile(&path),
        Command::Info { path } => info(&path),
        Command::Convert { input, output } => convert(&input, &output),
        Command::GenerateRoots {
            path,
            count,
            seed,
            ipa,
        } => generate_roots(&path, count, seed, ipa),
        Command::Features { path, ipa } => features(&path, ipa.as_deref()),
        Command::NewLexicon {
            path,
            seed,
            concepts,
            out,
        } => new_lexicon(&path, seed, concepts, out.as_deref()),
        Command::ExportMd { path, out } => export(&path, out.as_deref(), Rendering::Markdown),
        Command::ExportCsv { path, out } => export(&path, out.as_deref(), Rendering::Csv),
        Command::ApplyRules {
            path,
            rules,
            id,
            name,
            years,
            out,
        } => apply_rules(&path, &rules, &id, &name, years, out.as_deref()),
        Command::Trace { path, word } => trace(&path, &word),
        Command::Rules { path } => rules_summary(&path),
        Command::Fork {
            parent,
            id,
            name,
            rules,
            years,
            out,
        } => fork(&parent, &id, &name, rules.as_deref(), years, out.as_deref()),
        Command::Family { files } => family(&files),
        Command::Cognates { files, meanings } => cognates(&files, &meanings),
        Command::TraceWord { path, meaning } => trace_word(&path, &meaning),
        Command::Demo { out } => demo(out.as_deref()),
    }
}

/// Loads a genome, reporting which file failed if it does.
fn load_genome(path: &std::path::Path) -> Result<LanguageGenome> {
    stem_io::load(path).with_context(|| format!("loading language from `{}`", path.display()))
}

fn validate(path: &std::path::Path) -> Result<ExitCode> {
    let genome = load_genome(path)?;
    let report = genome.validate();

    println!("{}", genome.summary());
    println!();
    print_report(&report);

    // Errors mean the language is unusable; warnings are commentary. Only the
    // former should fail a script or a CI check.
    Ok(if report.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn profile(path: &std::path::Path) -> Result<ExitCode> {
    let genome = load_genome(path)?;
    let report = genome.validate();

    println!("{}", genome.summary());
    println!();
    // The scored-dimensions block is a derived read-model, rendered by the
    // library (the `render_family` precedent); the graded report — which already
    // carries the plausibility *warnings* — prints below it.
    print!(
        "{}",
        stem_genome::render_profile(&genome.plausibility_profile(), &genome.name)
    );
    println!();
    print_report(&report);

    Ok(if report.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn info(path: &std::path::Path) -> Result<ExitCode> {
    let genome = load_genome(path)?;

    println!("{}", genome.name);
    println!("{}", "=".repeat(genome.name.chars().count()));
    println!("id            {}", genome.id);
    println!(
        "lineage       {}",
        match &genome.parent {
            Some(parent) => format!("daughter of {parent}, +{}y", genome.lineage_depth_years),
            None => "proto-language".to_owned(),
        }
    );
    println!("seed          {}", genome.seed);
    println!();

    println!("phonotactics  {}", genome.phonotactics.summary());
    println!();

    println!("Phoneme inventory ({} total)", genome.phonemes.len());
    print_segments("  consonants  ", genome.phonemes.consonants());
    print_segments("  vowels      ", genome.phonemes.vowels());

    if !genome.notes.is_empty() {
        println!();
        println!("Notes");
        for note in &genome.notes {
            println!("  - {note}");
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn convert(input: &std::path::Path, output: &std::path::Path) -> Result<ExitCode> {
    let genome = load_genome(input)?;
    stem_io::save(output, &genome)
        .with_context(|| format!("writing language to `{}`", output.display()))?;
    println!("{} -> {}", input.display(), output.display());
    Ok(ExitCode::SUCCESS)
}

fn generate_roots(
    path: &std::path::Path,
    count: u64,
    seed_override: Option<u64>,
    as_ipa: bool,
) -> Result<ExitCode> {
    let genome = load_genome(path)?;

    // Seed precedence, pinned: `--seed` wins when given, otherwise the genome's own
    // seed. A file with no flag therefore always reproduces itself, which is what
    // `LanguageGenome::seed` promises — reproducible from the file alone.
    let (seed, provenance) = match seed_override {
        Some(seed) => (seed, "from --seed"),
        None => (genome.seed, "from the genome"),
    };

    let generator = RootGenerator::new(&genome.phonemes, &genome.phonotactics)
        .with_context(|| format!("generating roots for `{}`", genome.name))?;

    let mut rng = rng_for(seed, RngDomain::Roots);
    // `MAX_ROOTS` keeps this inside `usize` on every target this builds for.
    let roots = generator.generate(&mut rng, count as usize);

    // Everything explanatory goes to stderr so that stdout is exactly the roots,
    // one per line. That is what makes `diff <(run) <(run)` an honest determinism
    // check and the golden digest unambiguous.
    eprintln!("{} — seed {seed} ({provenance})", genome.name);
    eprintln!("phonotactics: {}", genome.phonotactics.summary());

    let mut lines = Vec::with_capacity(roots.len());
    for root in &roots {
        let form = if as_ipa {
            root.ipa(&genome.phonemes)
        } else {
            root.written(&genome.phonemes)
        }?;
        lines.push(form);
    }

    let unique: std::collections::BTreeSet<&String> = lines.iter().collect();
    if unique.len() < lines.len() {
        eprintln!(
            "note: {} of {} roots are duplicates (homophony is real; `stemma new-lexicon` reports it in the lexicon too)",
            lines.len() - unique.len(),
            lines.len()
        );
    }

    for line in lines {
        println!("{line}");
    }

    Ok(ExitCode::SUCCESS)
}

fn features(path: &std::path::Path, only_ipa: Option<&str>) -> Result<ExitCode> {
    let genome = load_genome(path)?;

    let mut shown = 0usize;
    for phoneme in genome.phonemes.iter() {
        if let Some(wanted) = only_ipa
            && phoneme.ipa != wanted
        {
            continue;
        }
        shown += 1;

        // Romanisation is a column, not a follow-on line, so every segment is one
        // self-contained record — greppable, and diffable against another language.
        let romanization = match &phoneme.romanization {
            Some(r) => format!("\"{r}\""),
            None => String::new(),
        };
        let rendered = if phoneme.features.is_empty() {
            "(no features declared)".to_owned()
        } else {
            phoneme.features.render()
        };
        println!(
            "/{}/{:pad$}  {:<10} {:<5} {}  w{:<4} {}",
            phoneme.ipa,
            "",
            phoneme.id.as_str(),
            romanization,
            phoneme.kind.template_symbol(),
            phoneme.frequency_weight,
            rendered,
            // Pad by character count, not byte length: IPA symbols are multi-byte
            // and `{:<n}` counts bytes, so the columns would drift without this.
            pad = 4usize.saturating_sub(phoneme.ipa.chars().count()),
        );
    }

    if shown == 0 {
        match only_ipa {
            Some(wanted) => anyhow::bail!("no phoneme with IPA form `{wanted}` in this language"),
            None => println!("(the inventory is empty)"),
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn new_lexicon(
    path: &std::path::Path,
    seed_override: Option<u64>,
    concepts: Option<u16>,
    out: Option<&std::path::Path>,
) -> Result<ExitCode> {
    let mut genome = load_genome(path)?;

    // `generate-roots`' precedence, verbatim: the flag wins when given, otherwise
    // the file's own seed, so a language always reproduces itself.
    let (seed, provenance) = match seed_override {
        Some(seed) => (seed, "from --seed"),
        None => (genome.seed, "from the genome"),
    };
    let count = concepts.map_or(CONCEPT_COUNT, |n| n as usize);

    let lexicon = build_proto_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        &CONCEPTS[..count],
        seed,
    )
    .with_context(|| format!("seeding a lexicon for `{}`", genome.name))?;

    eprintln!(
        "{} — seed {seed} ({provenance}), stream `lexicon`",
        genome.name
    );
    if !genome.lexicon.is_empty() {
        eprintln!(
            "note: replacing the existing lexicon of {}",
            genome.lexicon.summary()
        );
    }

    // Replace, never append. Appending is undefined and would produce a file the
    // validator immediately calls broken — duplicate word ids and duplicate
    // concepts — from a command that just reported success.
    genome.lexicon = lexicon;

    let homophones = stem_lexicon::check_against_inventory(&genome.lexicon, &genome.phonemes);
    for issue in homophones.issues.iter().filter(|i| i.code == "homophones") {
        eprintln!("note: {}", issue.message);
    }

    match out {
        Some(destination) => {
            // Not optional. `--seed 7 --out f.ron` must not write a file that says
            // `seed: 42` and holds a lexicon drawn from stream 7 — the genome's
            // seed promises reproducibility *from the file alone*, and this is the
            // first command in the project that persists a stochastic result.
            if genome.seed != seed {
                eprintln!(
                    "note: writing seed {seed} into the saved language, so it regenerates \
                     this lexicon from its own file"
                );
                genome.seed = seed;
            }
            stem_io::save(destination, &genome)
                .with_context(|| format!("writing `{}`", destination.display()))?;
            eprintln!("{} -> {}", genome.lexicon.summary(), destination.display());
        }
        None => {
            for entry in genome.lexicon.iter() {
                println!(
                    "{}\t{}\t{}",
                    entry.written(&genome.phonemes)?,
                    entry.display_gloss().unwrap_or("?"),
                    entry.concept.as_ref().map_or("", |k| k.as_str()),
                );
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn apply_rules(
    path: &std::path::Path,
    rules_path: &std::path::Path,
    id: &str,
    name: &str,
    years: i32,
    out: Option<&std::path::Path>,
) -> Result<ExitCode> {
    let genome = load_genome(path)?;
    let (evolved, report) = evolve_with_rules(&genome, rules_path, id, name, years)?;
    write_descendant(&genome, &evolved, &report, out)
}

/// Loads a rule set and evolves `parent` under it, reporting which files were
/// involved if it fails. Shared by `apply-rules` and `fork --rules`, so the two
/// verbs are the *same* operation on a language and cannot drift.
fn evolve_with_rules(
    parent: &LanguageGenome,
    rules_path: &std::path::Path,
    id: &str,
    name: &str,
    years: i32,
) -> Result<(LanguageGenome, ValidationReport)> {
    let rule_set: stem_soundchange::RuleSet = stem_io::load(rules_path)
        .with_context(|| format!("loading rules from `{}`", rules_path.display()))?;
    parent
        .evolve(id, name, &rule_set, years)
        .with_context(|| format!("evolving `{}` under `{}`", parent.name, rule_set.name))
}

/// Prints the run summary and report, then either writes the descendant (gated
/// on its own validation) or prints its lexicon as a TSV. **The write gate lives
/// here and only here** (`docs/adr/0008`): the descendant always exists and is
/// always reported, but a file that fails its own validation is never written —
/// that would persist a language `stemma validate` rejects.
fn write_descendant(
    parent: &LanguageGenome,
    descendant: &LanguageGenome,
    report: &ValidationReport,
    out: Option<&std::path::Path>,
) -> Result<ExitCode> {
    eprintln!(
        "{} -> {} — {} words, {} rules in history",
        parent.name,
        descendant.name,
        descendant.lexicon.len(),
        descendant.applied_rules.len(),
    );
    for issue in &report.issues {
        eprintln!("  {issue}");
    }

    match out {
        Some(destination) => {
            let verdict = descendant.validate();
            if !verdict.is_ok() {
                eprintln!("refusing to write `{}`:", destination.display());
                print_report(&verdict);
                return Ok(ExitCode::FAILURE);
            }
            stem_io::save(destination, descendant)
                .with_context(|| format!("writing `{}`", destination.display()))?;
            eprintln!("-> {}", destination.display());
        }
        None => {
            for entry in descendant.lexicon.iter() {
                println!(
                    "{}\t{}\t{}",
                    entry.written(&descendant.phonemes)?,
                    entry.display_gloss().unwrap_or("?"),
                    entry.id,
                );
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn fork(
    parent_path: &std::path::Path,
    id: &str,
    name: &str,
    rules: Option<&std::path::Path>,
    years: i32,
    out: Option<&std::path::Path>,
) -> Result<ExitCode> {
    let parent = load_genome(parent_path)?;

    // With rules, a fork IS an evolve — the same code path apply-rules takes.
    // Without, it is a verbatim relabelled copy; its report is just its own
    // validation, so a bare fork of a depth-0 proto honestly shows
    // `no_elapsed_time` before the gate.
    let (daughter, report) = match rules {
        Some(rules_path) => evolve_with_rules(&parent, rules_path, id, name, years)?,
        None => {
            let daughter = parent.fork(id, name, years);
            let report = daughter.validate();
            (daughter, report)
        }
    };

    write_descendant(&parent, &daughter, &report, out)
}

fn family(files: &[PathBuf]) -> Result<ExitCode> {
    let mut genomes = Vec::with_capacity(files.len());
    for file in files {
        genomes.push(load_genome(file)?);
    }

    let graph = stem_genome::LineageGraph::assemble(genomes);
    // Rendering is the library's job (the `render_derivation` precedent): the
    // M11 UI must produce identical text through this same function.
    print!("{}", stem_genome::render_family(&graph));

    // The report is printed separately from the rendering, so the snapshot test
    // pins only the tree + coverage.
    let report = graph.validate_family();
    println!();
    print_report(&report);

    Ok(if report.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn cognates(files: &[PathBuf], meanings: &[String]) -> Result<ExitCode> {
    let mut genomes = Vec::with_capacity(files.len());
    for file in files {
        genomes.push(load_genome(file)?);
    }
    let graph = stem_genome::LineageGraph::assemble(genomes);
    let table = graph
        .cognate_table(meanings)
        .context("building the cognate table")?;

    // Banner and notes to stderr; the table alone to stdout, so it stays
    // diffable (the `generate-roots` split).
    eprintln!(
        "reference: {} — meanings resolved here; * marks reconstructed root forms",
        table.reference
    );
    for note in &table.notes {
        eprintln!("note: {note}");
    }
    print!("{}", stem_genome::render_cognate_table(&table));

    Ok(ExitCode::SUCCESS)
}

fn trace_word(path: &std::path::Path, meaning: &str) -> Result<ExitCode> {
    let genome = load_genome(path)?;
    let matches = genome.lexicon.by_meaning(meaning);
    let entry = match matches.as_slice() {
        [] => anyhow::bail!("no word means `{meaning}` in `{}`", genome.name),
        [first, rest @ ..] => {
            if !rest.is_empty() {
                // Same first-in-stored-order policy the cognate table uses, so the
                // two can never point at different words for one meaning.
                eprintln!(
                    "note: {} words mean `{meaning}` in {}; tracing `{}`",
                    matches.len(),
                    genome.name,
                    first.id
                );
            }
            *first
        }
    };

    let rendered =
        stem_soundchange::render_derivation(entry, &genome.applied_rules, &genome.phonemes)
            .with_context(|| format!("rendering the derivation of `{}`", entry.id))?;
    print!("{rendered}");

    Ok(ExitCode::SUCCESS)
}

fn demo(out: Option<&std::path::Path>) -> Result<ExitCode> {
    use stem_io::Format;

    // The demo's inputs are compiled in, so it runs from any directory. Parsing
    // embedded RON is input sourcing, not domain logic — the story lives in
    // `stem_export::write_asterian_demo`.
    let proto: LanguageGenome = stem_io::load_str(
        include_str!("../../../fixtures/asterian_attested.ron"),
        Format::Ron,
    )
    .context("parsing the embedded proto fixture")?;
    let coastal: stem_soundchange::RuleSet = stem_io::load_str(
        include_str!("../../../fixtures/rules_coastal.ron"),
        Format::Ron,
    )
    .context("parsing the embedded Coastal rules")?;
    let highland: stem_soundchange::RuleSet = stem_io::load_str(
        include_str!("../../../fixtures/rules_highland.ron"),
        Format::Ron,
    )
    .context("parsing the embedded Highland rules")?;
    let riverine: stem_soundchange::RuleSet = stem_io::load_str(
        include_str!("../../../fixtures/rules_riverine.ron"),
        Format::Ron,
    )
    .context("parsing the embedded Riverine rules")?;

    let mut document = String::new();
    let report =
        stem_export::write_asterian_demo(&mut document, &proto, &coastal, &highland, &riverine)
            .context("assembling the family demo")?;
    // The advisory report (the velar-chain rules' `target_matches_nothing` notes)
    // goes to stderr; the document alone to the file or stdout.
    for issue in &report.issues {
        eprintln!("  {issue}");
    }

    match out {
        Some(destination) => {
            if let Some(parent) = destination.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating `{}`", parent.display()))?;
            }
            std::fs::write(destination, &document)
                .with_context(|| format!("writing `{}`", destination.display()))?;
            eprintln!("{} bytes -> {}", document.len(), destination.display());
        }
        // `print!`, not `println!`: the renderer already ends with a newline.
        None => print!("{document}"),
    }

    Ok(ExitCode::SUCCESS)
}

fn trace(path: &std::path::Path, word: &str) -> Result<ExitCode> {
    let genome = load_genome(path)?;
    let entry = genome
        .lexicon
        .require(&stem_core::WordId::new(word))
        .with_context(|| format!("looking up `{word}` in `{}`", genome.name))?;

    // The whole string is built by the library, so the M11 UI renders the same
    // text through the same function. The CLI contributes parsing and printing.
    let rendered =
        stem_soundchange::render_derivation(entry, &genome.applied_rules, &genome.phonemes)
            .with_context(|| format!("rendering the derivation of `{word}`"))?;
    print!("{rendered}");

    Ok(ExitCode::SUCCESS)
}

fn rules_summary(path: &std::path::Path) -> Result<ExitCode> {
    let rule_set: stem_soundchange::RuleSet =
        stem_io::load(path).with_context(|| format!("loading rules from `{}`", path.display()))?;

    println!("{} ({})", rule_set.name, rule_set.id);
    if !rule_set.description.is_empty() {
        println!("{}", rule_set.description);
    }
    println!();
    for (i, rule) in rule_set.rules.iter().enumerate() {
        println!(
            "  {i}  {}  {}  ({} years)",
            rule.id, rule.name, rule.chronology_years
        );
    }
    println!();

    let report = rule_set.validate();
    print_report(&report);
    Ok(if report.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Which document to render.
#[derive(Debug, Clone, Copy)]
enum Rendering {
    Markdown,
    Csv,
}

fn export(
    path: &std::path::Path,
    out: Option<&std::path::Path>,
    rendering: Rendering,
) -> Result<ExitCode> {
    let genome = load_genome(path)?;

    let mut document = String::new();
    match rendering {
        Rendering::Markdown => stem_export::write_lexicon_markdown(&mut document, &genome),
        Rendering::Csv => stem_export::write_lexicon_csv(&mut document, &genome),
    }
    .with_context(|| format!("rendering `{}`", genome.name))?;

    match out {
        Some(destination) => {
            if let Some(parent) = destination.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating `{}`", parent.display()))?;
            }
            std::fs::write(destination, &document)
                .with_context(|| format!("writing `{}`", destination.display()))?;
            eprintln!("{} bytes -> {}", document.len(), destination.display());
        }
        // `print!`, not `println!`: the renderers already end with a newline, and
        // an extra one would make stdout differ from the file byte for byte.
        None => print!("{document}"),
    }

    Ok(ExitCode::SUCCESS)
}

fn print_segments<'a>(label: &str, segments: impl Iterator<Item = &'a stem_phonology::Phoneme>) {
    let rendered: Vec<String> = segments.map(|p| format!("/{}/", p.ipa)).collect();
    if rendered.is_empty() {
        println!("{label}(none)");
    } else {
        println!("{label}{}", rendered.join(" "));
    }
}

fn print_report(report: &ValidationReport) {
    if report.is_empty() {
        println!("✓ no issues");
        return;
    }

    for issue in &report.issues {
        let mark = match issue.severity {
            Severity::Error => "✗",
            Severity::Warning => "!",
            Severity::Note => "·",
        };
        print!("{mark} {}: {}", issue.code, issue.message);
        match &issue.subject {
            Some(subject) => println!(" ({subject})"),
            None => println!(),
        }
    }

    println!();
    let errors = report.errors().count();
    let warnings = report.warnings().count();
    if errors == 0 {
        println!("✓ valid — {warnings} warning(s), nothing blocking");
    } else {
        println!("✗ invalid — {errors} error(s), {warnings} warning(s)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_cli_definition_is_well_formed() {
        // clap's own consistency check: catches duplicate flags, bad arg
        // configurations, and broken help output at test time rather than at
        // the user's terminal.
        Cli::command().debug_assert();
    }

    #[test]
    fn subcommands_parse() {
        let cli = Cli::parse_from(["stemma", "validate", "fixtures/proto_asterian.ron"]);
        assert!(matches!(cli.command, Command::Validate { .. }));

        let cli = Cli::parse_from(["stemma", "convert", "a.ron", "b.json"]);
        match cli.command {
            Command::Convert { input, output } => {
                assert_eq!(input, PathBuf::from("a.ron"));
                assert_eq!(output, PathBuf::from("b.json"));
            }
            other => panic!("expected convert, got {other:?}"),
        }

        // M4's two verbs.
        let cli = Cli::parse_from([
            "stemma",
            "fork",
            "proto.ron",
            "--id",
            "coastal",
            "--name",
            "Coastal",
            "--years",
            "470",
        ]);
        match cli.command {
            Command::Fork {
                id, name, years, ..
            } => {
                assert_eq!(id, "coastal");
                assert_eq!(name, "Coastal");
                assert_eq!(years, 470);
            }
            other => panic!("expected fork, got {other:?}"),
        }

        let cli = Cli::parse_from(["stemma", "family", "a.ron", "b.ron", "c.ron"]);
        match cli.command {
            Command::Family { files } => assert_eq!(files.len(), 3),
            other => panic!("expected family, got {other:?}"),
        }

        // M5's two verbs. `cognates` mixes a required variadic positional with a
        // greedy `--meanings` — the `--` split is what keeps clap from swallowing
        // the meanings as files.
        let cli = Cli::parse_from([
            "stemma",
            "cognates",
            "a.ron",
            "b.ron",
            "--meanings",
            "water",
            "sun",
        ]);
        match cli.command {
            Command::Cognates { files, meanings } => {
                assert_eq!(files.len(), 2, "two files");
                assert_eq!(meanings, vec!["water", "sun"], "two meanings");
            }
            other => panic!("expected cognates, got {other:?}"),
        }

        let cli = Cli::parse_from(["stemma", "trace-word", "coastal.ron", "star"]);
        match cli.command {
            Command::TraceWord { path, meaning } => {
                assert_eq!(path, PathBuf::from("coastal.ron"));
                assert_eq!(meaning, "star");
            }
            other => panic!("expected trace-word, got {other:?}"),
        }

        let cli = Cli::parse_from(["stemma", "demo", "--out", "output/demo.md"]);
        match cli.command {
            Command::Demo { out } => assert_eq!(out, Some(PathBuf::from("output/demo.md"))),
            other => panic!("expected demo, got {other:?}"),
        }

        let cli = Cli::parse_from(["stemma", "profile", "x.ron"]);
        match cli.command {
            Command::Profile { path } => assert_eq!(path, PathBuf::from("x.ron")),
            other => panic!("expected profile, got {other:?}"),
        }
    }

    /// Both time-advancing verbs reject a negative `--years=` at parse time —
    /// time runs forwards, and a negative elapsed value on a deep parent would
    /// slip the genome gate (`negative_lineage_depth` needs *total* < 0), then
    /// persist through the shared write gate. The two must agree.
    #[test]
    fn fork_and_apply_rules_reject_negative_years() {
        let fork = Cli::try_parse_from([
            "stemma",
            "fork",
            "proto.ron",
            "--id",
            "x",
            "--name",
            "X",
            "--years=-5",
        ]);
        assert!(fork.is_err(), "fork must reject negative years");

        let apply = Cli::try_parse_from([
            "stemma",
            "apply-rules",
            "proto.ron",
            "--rules",
            "r.ron",
            "--id",
            "x",
            "--name",
            "X",
            "--years=-5",
        ]);
        assert!(apply.is_err(), "apply-rules must reject negative years too");
    }
}
