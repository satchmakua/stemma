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
        Command::Info { path } => info(&path),
        Command::Convert { input, output } => convert(&input, &output),
        Command::GenerateRoots {
            path,
            count,
            seed,
            ipa,
        } => generate_roots(&path, count, seed, ipa),
        Command::Features { path, ipa } => features(&path, ipa.as_deref()),
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
            "note: {} of {} roots are duplicates (homophony is real; --unique lands in M2)",
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
    }
}
