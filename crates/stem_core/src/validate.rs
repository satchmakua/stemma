//! Validation as a *report*, not a boolean.
//!
//! Stemma checks languages the way a linguist would: some things are broken (a
//! word referencing a phoneme that isn't in the inventory), and some things are
//! merely unusual (an inventory with 80 consonants and 2 vowels). Collapsing both
//! into `valid: bool` would force the engine to either reject legitimate
//! speculative designs or say nothing useful about them.
//!
//! So validation returns a [`ValidationReport`] — a list of graded [`Issue`]s.
//! [`Severity::Error`] blocks the pipeline; [`Severity::Warning`] and
//! [`Severity::Note`] are advisory. This is the foundation the plausibility
//! profile of `DESIGN.md` §17 is built on: that feature is this report with more
//! checks behind it, not a separate subsystem.

use std::fmt;

use serde::{Deserialize, Serialize};

/// How much a validation issue matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Advisory context — no action implied.
    Note,
    /// Typologically odd or historically unmotivated, but permitted.
    Warning,
    /// Structurally broken; the engine cannot proceed.
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Note => "note",
            Self::Warning => "warning",
            Self::Error => "error",
        };
        f.write_str(s)
    }
}

/// A single finding about a language or one of its parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    /// How much it matters.
    pub severity: Severity,
    /// A stable machine-readable code, e.g. `"phonology.duplicate_id"`.
    ///
    /// Codes are stable so that tests, exports, and (later) the UI can key off
    /// them without matching on prose.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
    /// What the issue is about — an ID, a phoneme, a rule name.
    pub subject: Option<String>,
}

impl Issue {
    /// Builds an issue at the given severity.
    pub fn new(severity: Severity, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            subject: None,
        }
    }

    /// Attaches the subject this issue is about.
    #[must_use]
    pub fn about(mut self, subject: impl fmt::Display) -> Self {
        self.subject = Some(subject.to_string());
        self
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: [{}] {}", self.severity, self.code, self.message)?;
        if let Some(subject) = &self.subject {
            write!(f, " ({subject})")?;
        }
        Ok(())
    }
}

/// The result of validating something: an ordered list of issues.
///
/// Order is preserved — checks run in a deliberate order and the report reads
/// like a walkthrough of the language.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Every issue found, in the order the checks ran.
    pub issues: Vec<Issue>,
}

impl ValidationReport {
    /// An empty report — nothing wrong, nothing noted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an error. The pipeline should not continue past one of these.
    pub fn error(&mut self, code: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.issues.push(Issue::new(Severity::Error, code, message));
        self
    }

    /// Records a warning: permitted, but worth a historical explanation.
    pub fn warn(&mut self, code: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.issues
            .push(Issue::new(Severity::Warning, code, message));
        self
    }

    /// Records an advisory note.
    pub fn note(&mut self, code: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.issues.push(Issue::new(Severity::Note, code, message));
        self
    }

    /// Pushes a pre-built issue (use when it carries a subject).
    pub fn push(&mut self, issue: Issue) -> &mut Self {
        self.issues.push(issue);
        self
    }

    /// Absorbs another report, prefixing every code with `scope`.
    ///
    /// This is how an aggregate delegates: a genome validates its inventory and
    /// merges the result under `"phonology"`, so codes stay namespaced and the
    /// origin of an issue is never ambiguous.
    pub fn absorb(&mut self, scope: &str, other: ValidationReport) -> &mut Self {
        self.issues
            .extend(other.issues.into_iter().map(|mut issue| {
                issue.code = format!("{scope}.{}", issue.code);
                issue
            }));
        self
    }

    /// Issues at [`Severity::Error`].
    pub fn errors(&self) -> impl Iterator<Item = &Issue> {
        self.of_severity(Severity::Error)
    }

    /// Issues at [`Severity::Warning`].
    pub fn warnings(&self) -> impl Iterator<Item = &Issue> {
        self.of_severity(Severity::Warning)
    }

    /// Issues at exactly the given severity.
    pub fn of_severity(&self, severity: Severity) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(move |i| i.severity == severity)
    }

    /// True when nothing blocks the pipeline. Warnings do not make a language
    /// invalid — that is the whole point of grading them.
    pub fn is_ok(&self) -> bool {
        self.errors().next().is_none()
    }

    /// True when there is nothing to report at all.
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// How many issues were found.
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Converts a report with errors into an [`crate::StemmaError::Invalid`].
    ///
    /// Callers that genuinely cannot proceed use this; callers that want to show
    /// the user a profile keep the report.
    pub fn into_result(self, subject: impl Into<String>) -> crate::Result<Self> {
        if self.is_ok() {
            Ok(self)
        } else {
            Err(crate::StemmaError::Invalid(subject.into(), self))
        }
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.issues.is_empty() {
            return f.write_str("no issues");
        }
        for (i, issue) in self.issues.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "  {issue}")?;
        }
        Ok(())
    }
}

/// Anything that can be checked for structural integrity and plausibility.
pub trait Validate {
    /// Runs every check and returns the findings.
    ///
    /// Implementations must not stop at the first problem — the report is meant
    /// to be a complete picture, so a user fixes everything in one pass.
    fn validate(&self) -> ValidationReport;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_alone_do_not_make_something_invalid() {
        let mut report = ValidationReport::new();
        report.warn("phonology.lopsided_inventory", "80 consonants, 2 vowels");
        assert!(report.is_ok(), "a warning must not block the pipeline");
        assert_eq!(report.warnings().count(), 1);
        assert_eq!(report.errors().count(), 0);
    }

    #[test]
    fn errors_make_something_invalid() {
        let mut report = ValidationReport::new();
        report.error("phonology.no_vowels", "inventory has no syllable nucleus");
        assert!(!report.is_ok());
        assert!(report.clone().into_result("Proto-Asterian").is_err());
    }

    #[test]
    fn absorb_namespaces_the_codes_of_the_inner_report() {
        let mut inner = ValidationReport::new();
        inner.error("duplicate_id", "two phonemes share an id");

        let mut outer = ValidationReport::new();
        outer.absorb("phonology", inner);

        assert_eq!(outer.issues[0].code, "phonology.duplicate_id");
    }

    #[test]
    fn absorb_preserves_severity_and_subject() {
        let mut inner = ValidationReport::new();
        inner.push(Issue::new(Severity::Warning, "odd", "unusual").about("ph_0001"));

        let mut outer = ValidationReport::new();
        outer.absorb("phonology", inner);

        assert_eq!(outer.issues[0].severity, Severity::Warning);
        assert_eq!(outer.issues[0].subject.as_deref(), Some("ph_0001"));
    }

    #[test]
    fn an_empty_report_displays_as_no_issues() {
        assert_eq!(ValidationReport::new().to_string(), "no issues");
    }
}
