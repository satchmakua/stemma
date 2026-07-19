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
use stem_core::{Severity, Validate, ValidationReport};
use stem_genome::LanguageGenome;

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
