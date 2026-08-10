//! §11.1's readable rule syntax, as a **parser over the M3 structs** (ROADMAP M10).
//!
//! # This is a front end, never a second engine
//!
//! Every path here ends in a [`SoundChangeRule`] — the same struct a `.ron` rule
//! file deserialises to, handed to the same unchanged `apply_rules`. There is no
//! DSL-specific matching, no DSL-specific application, and no behaviour reachable
//! from this syntax that the structs cannot already express. The milestone's
//! acceptance is exactly that claim made checkable: the M3 rule set written in this
//! syntax produces **byte-identical** output to the hand-built structs.
//!
//! That ordering is deliberate (§20.4): structs first at M3, parser later at M10, so
//! the syntax was designed against semantics that already worked rather than the
//! reverse. A DSL invented first tends to acquire features the engine then has to
//! grow to match.
//!
//! # The syntax
//!
//! ```text
//! rules rules_asterian_early "Early Asterian sound changes":
//!   note: "The chain of DESIGN.md §10.2, plus nasal place assimilation."
//!
//! rule r_0001 "Intervocalic voicing":
//!   note: "Voiceless stops voice between vowels."
//!   at: 250
//!   target: [-sonorant, -continuant, -voice]
//!   environment: [+syllabic] _ [+syllabic]
//!   change: set [+voice]
//!
//! rule r_0003 "Final unstressed vowel loss":
//!   at: 410
//!   target: [+syllabic, unstressed]
//!   environment: _ #
//!   change: delete
//!
//! rule r_0002 "Nasal place assimilation":
//!   at: 300
//!   target: [+nasal]
//!   environment: _ [-sonorant, -continuant]
//!   change: copy place from after[0]
//! ```
//!
//! `//` begins a comment — `#` is taken, it is the word boundary. Blank lines are
//! ignored. Rule order in the file **is**
//! chronology, exactly as the `Vec` order is in a `.ron` set (§11.3).
//!
//! # Three deliberate deviations from §11.1's sketch
//!
//! 1. **A rule carries an id *and* a name.** §11.1 writes `rule IntervocalicVoicing:`
//!    — one token doing both jobs. The engine needs a stable [`RuleId`] that traces
//!    reference and a human name that derivations print, and `docs/adr/0003` requires
//!    ids be readable and stable rather than derived from a display string that an
//!    author may reword. So: `rule r_0001 "Intervocalic voicing":`.
//! 2. **`change: set [+voice]`, not §11.1's `voice = true`.** The bracket already
//!    means "a bundle of valued cells" in `target:`; reusing it makes a multi-cell
//!    change (`set [+voice, +continuant]`) fall out, where `voice = true` would need
//!    a second comma syntax that means the same thing. One notation, one meaning.
//! 3. **`copy` exists at all.** §11.1 has no syntax for it, because every example
//!    there sets a literal — and `rule.rs` records that a model designed from §11.1
//!    alone gets this wrong. §7.2's "nasals assimilate to the place of a following
//!    stop" is not `place = labial`; it is *whatever the next segment's place is*.
//!    Without `copy` the DSL could not express one of the four reference rules.
//!
//! # Feature abbreviations
//!
//! `V` and `C` are accepted where a bracket is, expanding to `[+syllabic]` and
//! `[-syllabic]` — §11.1 writes `environment: V _ V`. They are **aliases for feature
//! bundles**, not letter classes: `V` is not "a, e, i, o, u", it is the natural class
//! `[+syllabic]`, which is why a segment a rule *innovates* falls into it
//! automatically. The features-not-letters rule (§7.1) is intact, and
//! `v_is_exactly_the_syllabic_natural_class` pins it.

use stem_core::{Result, RuleId, StemmaError};
use stem_phonology::prosody::Stress;
use stem_phonology::{FeatureBundle, FeatureNode};

use crate::rule::{
    Change, EnvItem, Environment, Position, RuleSet, SegmentPattern, SoundChangeRule,
};

/// Parses a rule set written in §11.1's syntax.
///
/// `path` is used only to name the file in errors; it is never opened. Errors carry
/// a **line number and the offending text**, because a parse failure a user cannot
/// locate is a parse failure they cannot fix.
pub fn parse_rule_set(source: &str, path: &str) -> Result<RuleSet> {
    Parser {
        path,
        lines: source.lines().enumerate(),
    }
    .run()
}

/// A parse error, with the line it happened on.
fn err(path: &str, line: usize, message: impl std::fmt::Display) -> StemmaError {
    StemmaError::Parse {
        path: path.to_owned(),
        format: "rule DSL",
        message: format!("line {line}: {message}"),
    }
}

struct Parser<'a> {
    path: &'a str,
    lines: std::iter::Enumerate<std::str::Lines<'a>>,
}

/// One `key: value` line, with its source line number.
struct Field<'a> {
    key: &'a str,
    value: &'a str,
    line: usize,
}

impl<'a> Parser<'a> {
    fn run(mut self) -> Result<RuleSet> {
        let mut header: Option<(String, String, usize)> = None;
        let mut rules: Vec<SoundChangeRule> = Vec::new();
        // The block currently being filled: its id, name, opening line, and fields.
        let mut open: Option<(String, String, usize, Vec<Field<'a>>)> = None;
        let mut set_note = String::new();

        while let Some((n, raw)) = self.lines.next() {
            let line = n + 1;
            let text = strip_comment(raw).trim_end();
            if text.trim().is_empty() {
                continue;
            }

            if let Some(rest) = text.trim().strip_prefix("rules ") {
                if header.is_some() {
                    return Err(err(
                        self.path,
                        line,
                        "a second `rules` header; a file declares one set",
                    ));
                }
                if open.is_some() {
                    return Err(err(
                        self.path,
                        line,
                        "`rules` header must come before any rule",
                    ));
                }
                let (id, name) = self.id_and_name(rest, line)?;
                header = Some((id, name, line));
                continue;
            }

            if let Some(rest) = text.trim().strip_prefix("rule ") {
                if let Some(block) = open.take() {
                    rules.push(self.finish_rule(block)?);
                }
                let (id, name) = self.id_and_name(rest, line)?;
                open = Some((id, name, line, Vec::new()));
                continue;
            }

            // Otherwise it is a `key: value` field belonging to whatever block is
            // open — the rule if one is, else the set header.
            let (key, value) = text.split_once(':').ok_or_else(|| {
                err(
                    self.path,
                    line,
                    format!("expected `key: value`, found `{}`", text.trim()),
                )
            })?;
            let field = Field {
                key: key.trim(),
                value: value.trim(),
                line,
            };
            match (&mut open, &header) {
                (Some((_, _, _, fields)), _) => fields.push(field),
                (None, Some(_)) => {
                    if field.key == "note" {
                        set_note = unquote(field.value, self.path, line)?;
                    } else {
                        return Err(err(
                            self.path,
                            line,
                            format!("unknown set field `{}`", field.key),
                        ));
                    }
                }
                (None, None) => {
                    return Err(err(
                        self.path,
                        line,
                        "a field before any `rules` header or `rule`",
                    ));
                }
            }
        }
        if let Some(block) = open.take() {
            rules.push(self.finish_rule(block)?);
        }

        let (id, name, _) = header.ok_or_else(|| {
            err(
                self.path,
                1,
                "no `rules <id> \"<name>\":` header — a set needs an identity",
            )
        })?;
        Ok(RuleSet {
            id,
            name,
            description: set_note,
            rules,
        })
    }

    /// `r_0001 "Intervocalic voicing":` → (`r_0001`, `Intervocalic voicing`).
    fn id_and_name(&self, rest: &str, line: usize) -> Result<(String, String)> {
        let rest = rest
            .trim()
            .strip_suffix(':')
            .ok_or_else(|| err(self.path, line, "a `rules`/`rule` line ends with `:`"))?;
        let (id, quoted) = rest.trim().split_once(' ').ok_or_else(|| {
            err(
                self.path,
                line,
                format!("expected `<id> \"<name>\":`, found `{}`", rest.trim()),
            )
        })?;
        Ok((
            id.trim().to_owned(),
            unquote(quoted.trim(), self.path, line)?,
        ))
    }

    fn finish_rule(
        &self,
        (id, name, open_line, fields): (String, String, usize, Vec<Field<'a>>),
    ) -> Result<SoundChangeRule> {
        let mut description = String::new();
        let mut chronology_years: i32 = 0;
        let mut target: Option<SegmentPattern> = None;
        let mut environment = Environment::default();
        let mut change: Option<Change> = None;

        for field in fields {
            match field.key {
                "note" => description = unquote(field.value, self.path, field.line)?,
                "at" => {
                    let digits = field.value.trim().trim_end_matches('y');
                    chronology_years = digits.trim().parse().map_err(|_| {
                        err(
                            self.path,
                            field.line,
                            format!("`at:` wants a year, found `{}`", field.value),
                        )
                    })?;
                }
                "target" => target = Some(self.segment_pattern(field.value, field.line)?),
                "environment" => environment = self.environment(field.value, field.line)?,
                "change" => change = Some(self.change(field.value, field.line)?),
                other => {
                    return Err(err(
                        self.path,
                        field.line,
                        format!(
                            "unknown rule field `{other}` (expected note, at, target, environment, change)"
                        ),
                    ));
                }
            }
        }

        Ok(SoundChangeRule {
            id: RuleId::new(id),
            name,
            description,
            chronology_years,
            target: target
                .ok_or_else(|| err(self.path, open_line, "this rule declares no `target:`"))?,
            environment,
            change: change
                .ok_or_else(|| err(self.path, open_line, "this rule declares no `change:`"))?,
        })
    }

    /// `[+syllabic, unstressed]`, or the `V` / `C` aliases.
    ///
    /// Stress words inside the bracket land on the pattern's **stress** axis rather
    /// than its features — §11.1 writes `[+vowel, -stress]`, and stress is
    /// syllable-scoped while features are segment-scoped, so the two halves of that
    /// bracket genuinely belong on two axes.
    fn segment_pattern(&self, text: &str, line: usize) -> Result<SegmentPattern> {
        let text = text.trim();
        let inner = match text {
            "V" => "+syllabic".to_owned(),
            "C" => "-syllabic".to_owned(),
            _ => text
                .strip_prefix('[')
                .and_then(|t| t.strip_suffix(']'))
                .ok_or_else(|| {
                    err(
                        self.path,
                        line,
                        format!("expected `[features]`, `V` or `C`, found `{text}`"),
                    )
                })?
                .to_owned(),
        };

        let mut features: Vec<String> = Vec::new();
        let mut stress: Option<Stress> = None;
        for token in inner.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            match token {
                "unstressed" => stress = Some(Stress::Unstressed),
                "stressed" | "primary" => stress = Some(Stress::Primary),
                feature => features.push(feature.to_owned()),
            }
        }
        // The feature names come from `stem_phonology`'s own converter — there is no
        // second table of feature spellings here, so a name this parser accepts is
        // exactly a name the engine accepts.
        let features = FeatureBundle::try_from(features)
            .map_err(|e| err(self.path, line, format!("in `{text}`: {e}")))?;
        Ok(SegmentPattern { features, stress })
    }

    /// `[+syllabic] _ [+syllabic]`, `_ #`, `# V _`.
    ///
    /// The underscore is the target. Items are written **left to right as they
    /// appear in the word**, which means the left context has to be reversed on the
    /// way in: `Environment::before` is stored *outward from the target*, so
    /// `A B _` has `before[0] == B`, the segment immediately left.
    fn environment(&self, text: &str, line: usize) -> Result<Environment> {
        let (left, right) = text.split_once('_').ok_or_else(|| {
            err(
                self.path,
                line,
                format!("an environment needs a `_` for the target, found `{text}`"),
            )
        })?;
        let mut before = self.env_items(left, line)?;
        before.reverse();
        Ok(Environment {
            before,
            after: self.env_items(right, line)?,
        })
    }

    fn env_items(&self, text: &str, line: usize) -> Result<Vec<EnvItem>> {
        let mut items = Vec::new();
        for token in split_env(text) {
            items.push(match token.as_str() {
                "#" => EnvItem::Boundary,
                pattern => EnvItem::Segment(self.segment_pattern(pattern, line)?),
            });
        }
        Ok(items)
    }

    /// `set [+voice]`, `copy place from after[0]`, `delete`.
    fn change(&self, text: &str, line: usize) -> Result<Change> {
        let text = text.trim();
        if text == "delete" {
            return Ok(Change::Delete);
        }
        if let Some(rest) = text.strip_prefix("set ") {
            return Ok(Change::Set(self.segment_pattern(rest, line)?.features));
        }
        if let Some(rest) = text.strip_prefix("copy ") {
            let (node, from) = rest.split_once(" from ").ok_or_else(|| {
                err(
                    self.path,
                    line,
                    format!("expected `copy <node> from <position>`, found `{text}`"),
                )
            })?;
            let node = match node.trim() {
                "place" => FeatureNode::Place,
                "laryngeal" => FeatureNode::Laryngeal,
                other => {
                    return Err(err(
                        self.path,
                        line,
                        format!("unknown feature node `{other}` (expected place or laryngeal)"),
                    ));
                }
            };
            return Ok(Change::Copy {
                from: self.position(from.trim(), line)?,
                node,
            });
        }
        Err(err(
            self.path,
            line,
            format!("expected `set [...]`, `copy ... from ...` or `delete`, found `{text}`"),
        ))
    }

    /// `after[0]` / `before[1]` — an index into the environment slot of that side,
    /// mirroring `Position::After(0)` exactly rather than inventing a
    /// one-based "the segment after" that would read the same and mean something
    /// else.
    fn position(&self, text: &str, line: usize) -> Result<Position> {
        let parse_index = |inner: &str| -> Result<u8> {
            inner.trim().parse::<u8>().map_err(|_| {
                err(
                    self.path,
                    line,
                    format!("`{text}` needs a slot index, e.g. `after[0]`"),
                )
            })
        };
        if let Some(inner) = text
            .strip_prefix("after[")
            .and_then(|t| t.strip_suffix(']'))
        {
            return Ok(Position::After(parse_index(inner)?));
        }
        if let Some(inner) = text
            .strip_prefix("before[")
            .and_then(|t| t.strip_suffix(']'))
        {
            return Ok(Position::Before(parse_index(inner)?));
        }
        Err(err(
            self.path,
            line,
            format!("expected `after[n]` or `before[n]`, found `{text}`"),
        ))
    }
}

/// Everything before an unquoted `//`.
///
/// **The comment marker is `//`, not `#`** — and that is forced, not a preference.
/// §11.1 gives `#` to the word boundary, so a line-leading `# a note` is
/// indistinguishable from a standalone boundary symbol by any *local* rule: both
/// are a `#` surrounded by whitespace. Every heuristic that separates them ("does
/// the rest of the line parse as environment items?", "are we inside an
/// `environment:` value?") makes the lexer depend on the parser, which is how a
/// comment starts silently changing a rule. `//` collides with nothing, and matches
/// the language the rest of this repository is written in.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_quotes = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            '/' if !in_quotes && bytes.get(i + 1) == Some(&b'/') => return &line[..i],
            _ => {}
        }
    }
    line
}

/// Splits an environment side into `[...]`, `V`, `C` and `#` tokens.
fn split_env(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('[') {
            match after.find(']') {
                Some(close) => {
                    out.push(rest[..close + 2].to_owned());
                    rest = rest[close + 2..].trim_start();
                }
                // Unterminated: hand the whole thing on so `segment_pattern`
                // reports it with the real text rather than a truncated fragment.
                None => {
                    out.push(rest.to_owned());
                    break;
                }
            }
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push(rest[..end].to_owned());
            rest = rest[end..].trim_start();
        }
    }
    out
}

/// Strips surrounding double quotes.
fn unquote(text: &str, path: &str, line: usize) -> Result<String> {
    let text = text.trim();
    text.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| {
            err(
                path,
                line,
                format!("expected a quoted string, found `{text}`"),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Result<RuleSet> {
        parse_rule_set(source, "test.sc")
    }

    fn one_rule(body: &str) -> Result<SoundChangeRule> {
        let source = format!("rules s \"S\":\n\nrule r_1 \"R\":\n{body}\n");
        Ok(parse(&source)?.rules.into_iter().next().expect("one rule"))
    }

    #[test]
    fn a_set_header_carries_its_id_name_and_note() {
        let set = parse("rules rs \"The set\":\n  note: \"why\"\n").expect("parses");
        assert_eq!(set.id, "rs");
        assert_eq!(set.name, "The set");
        assert_eq!(set.description, "why");
        assert!(set.rules.is_empty());
    }

    #[test]
    fn intervocalic_voicing_parses_to_the_hand_built_struct() {
        let rule = one_rule(
            "  note: \"Voiceless stops voice between vowels.\"\n  \
               at: 250\n  \
               target: [-sonorant, -continuant, -voice]\n  \
               environment: [+syllabic] _ [+syllabic]\n  \
               change: set [+voice]",
        )
        .expect("parses");
        assert_eq!(rule.id.as_str(), "r_1");
        assert_eq!(rule.chronology_years, 250);
        assert_eq!(rule.environment.before.len(), 1);
        assert_eq!(rule.environment.after.len(), 1);
        assert!(matches!(rule.change, Change::Set(_)));
    }

    /// The left context is written left-to-right but **stored outward from the
    /// target**, so `A B _` must yield `before[0] == B`.
    #[test]
    fn the_left_context_is_reversed_into_outward_order() {
        let rule = one_rule("  target: [+syllabic]\n  environment: # V _\n  change: delete")
            .expect("parses");
        assert_eq!(
            rule.environment.before.len(),
            2,
            "both items are kept: {:?}",
            rule.environment.before
        );
        assert!(
            matches!(rule.environment.before[0], EnvItem::Segment(_)),
            "the vowel is immediately left of the target, so it is before[0]"
        );
        assert!(
            matches!(rule.environment.before[1], EnvItem::Boundary),
            "the boundary is further out, so it is before[1]"
        );
    }

    #[test]
    fn a_boundary_on_the_right_parses() {
        let rule = one_rule("  target: [+syllabic]\n  environment: _ #\n  change: delete")
            .expect("parses");
        assert_eq!(rule.environment.after, vec![EnvItem::Boundary]);
        assert!(rule.environment.before.is_empty());
    }

    #[test]
    fn a_stress_word_lands_on_the_stress_axis_not_the_features() {
        let rule =
            one_rule("  target: [+syllabic, unstressed]\n  environment: _ #\n  change: delete")
                .expect("parses");
        assert_eq!(rule.target.stress, Some(Stress::Unstressed));
        // And the feature half survived alongside it.
        assert!(!rule.target.features.is_empty());
    }

    #[test]
    fn copy_names_a_node_and_an_environment_slot() {
        let rule = one_rule(
            "  target: [+nasal]\n  \
               environment: _ [-sonorant, -continuant]\n  \
               change: copy place from after[0]",
        )
        .expect("parses");
        assert_eq!(
            rule.change,
            Change::Copy {
                from: Position::After(0),
                node: FeatureNode::Place
            }
        );
    }

    /// `V` is an alias for a **feature bundle**, not a list of letters — which is
    /// what keeps §7.1 intact while still writing §11.1's `V _ V`.
    #[test]
    fn v_is_exactly_the_syllabic_natural_class() {
        let with_alias = one_rule("  target: V\n  environment: _ #\n  change: delete").unwrap();
        let spelled_out =
            one_rule("  target: [+syllabic]\n  environment: _ #\n  change: delete").unwrap();
        assert_eq!(with_alias.target, spelled_out.target);

        let c = one_rule("  target: C\n  environment: _ #\n  change: delete").unwrap();
        let minus =
            one_rule("  target: [-syllabic]\n  environment: _ #\n  change: delete").unwrap();
        assert_eq!(c.target, minus.target);
    }

    #[test]
    fn a_comment_is_stripped_but_a_boundary_symbol_is_not() {
        let rule = one_rule(
            "  // this whole line is a comment\n  \
               target: [+syllabic]\n  \
               environment: _ #   // and this is a trailing one\n  \
               change: delete",
        )
        .expect("parses");
        assert_eq!(
            rule.environment.after,
            vec![EnvItem::Boundary],
            "`//` comments out the rest of a line; a bare `#` stays the word edge"
        );
    }

    // --- errors name the line and the text ---

    #[test]
    fn an_unknown_feature_names_the_line_and_the_bracket() {
        let e = one_rule("  target: [+sylabic]\n  environment: _ #\n  change: delete")
            .expect_err("misspelled feature");
        let text = e.to_string();
        assert!(text.contains("line 4"), "names the line: {text}");
        assert!(text.contains("sylabic"), "names the offending text: {text}");
    }

    #[test]
    fn a_missing_target_is_reported_against_the_rule_line() {
        let e = one_rule("  environment: _ #\n  change: delete").expect_err("no target");
        assert!(e.to_string().contains("no `target:`"), "{e}");
    }

    #[test]
    fn an_environment_with_no_underscore_is_rejected() {
        let e = one_rule("  target: V\n  environment: V V\n  change: delete")
            .expect_err("no target marker");
        assert!(e.to_string().contains('_'), "{e}");
    }

    #[test]
    fn an_unknown_field_is_rejected_rather_than_ignored() {
        let e = one_rule("  target: V\n  environment: _ #\n  chnage: delete")
            .expect_err("misspelled field");
        assert!(e.to_string().contains("chnage"), "{e}");
    }

    #[test]
    fn a_file_with_no_header_is_rejected() {
        assert!(parse("rule r_1 \"R\":\n  target: V\n").is_err());
    }
}
