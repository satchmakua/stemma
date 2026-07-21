//! A family of languages, assembled in memory from loaded genome files.
//!
//! **The `parent` field on each genome is the single source of truth for
//! descent.** Edges are *derived* from it on demand and never stored; a stored
//! edge list beside the `parent` fields would be a second copy of the same fact
//! that nothing could keep synchronised — the defect class this project bans
//! three times over (`form` beside `phonemic_form`, a stored intermediate form,
//! `Syllable::pattern`-as-semantics). See `docs/adr/0008`.
//!
//! There is **no map** anywhere in this module. A `HashMap` would leak iteration
//! order toward output (§9.4's determinism rule) and silently swallow the
//! duplicate ids `docs/adr/0003` requires the validator to see. Nodes are a
//! `Vec` in the order the caller gave (argv order at the CLI — the authored-order
//! rule of M1, applied to files), and every lookup is a linear scan. A family is
//! tens of nodes; the scan is not the bottleneck and never will be.
//!
//! **There is no `LineageEdgeKind` enum** — not even `Descent`. With edges
//! derived rather than persisted there is no file format to stabilise, so adding
//! kinds later is a pure code change, and a one-variant enum is scaffolding by
//! the `HistoricalEvent` precedent. A dialect split *is* descent; "split" is
//! topology (out-degree > 1) and derivable. The contact-like kinds
//! (`DESIGN.md` §8.6) are not even derivable from `parent` — a contact edge is a
//! *second* parent — and arrive with their producers (M7+) as additive genome
//! fields.

use stem_core::{CognateSetId, Issue, LanguageId, Severity, Validate, ValidationReport, WordId};

use crate::LanguageGenome;

/// A family of languages. Never persisted. Never sorted. `nodes` keeps the order
/// the caller gave.
#[derive(Debug, Clone)]
pub struct LineageGraph {
    nodes: Vec<LanguageGenome>,
}

/// One derived descent relationship: borrowed views, computed on demand. No
/// `kind` field — see the module docs and `docs/adr/0008`.
#[derive(Debug, Clone, Copy)]
pub struct LineageEdge<'a> {
    pub parent: &'a LanguageGenome,
    pub child: &'a LanguageGenome,
}

impl LineageEdge<'_> {
    /// `child.lineage_depth_years - parent.lineage_depth_years`. This delta —
    /// never the child's total depth — is what [`render_family`] prints as
    /// `+Ny`. A negative delta is reported as `family.depth_regression`.
    pub fn elapsed_years(&self) -> i32 {
        self.child.lineage_depth_years - self.parent.lineage_depth_years
    }
}

/// Coverage of one root's cognate sets across its transitive descendants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CognateCoverage {
    /// The root whose lexicon defines the sets counted.
    pub root: LanguageId,
    /// Transitive descendants of the root (parent-link closure), count.
    pub descendants: usize,
    /// Distinct cognate sets in the root's lexicon.
    pub sets: usize,
    /// Sets present in *every* descendant.
    pub universal: usize,
    /// Sets absent from at least one descendant, each with who lacks it. In the
    /// root lexicon's authored order; languages in node order.
    pub gaps: Vec<(CognateSetId, Vec<LanguageId>)>,
}

impl LineageGraph {
    /// Assembles a family from loaded genomes, keeping their given order. Never
    /// fails: a broken family is a *report* ([`Self::validate_family`]), not a
    /// refusal (§17).
    pub fn assemble(nodes: Vec<LanguageGenome>) -> Self {
        Self { nodes }
    }

    /// How many languages the family holds.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the family holds no languages.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The languages, in the order the caller gave.
    pub fn nodes(&self) -> &[LanguageGenome] {
        &self.nodes
    }

    /// The first language with this id, in stored order. Deterministic under
    /// duplicate ids — which [`Self::validate_family`] reports as an Error.
    pub fn get(&self, id: &LanguageId) -> Option<&LanguageGenome> {
        self.nodes.iter().find(|g| &g.id == id)
    }

    /// The stored index of the first language with this id.
    fn index_of(&self, id: &LanguageId) -> Option<usize> {
        self.nodes.iter().position(|g| &g.id == id)
    }

    /// The stored index of node `i`'s parent, if the parent is present in the
    /// assembly. Resolves by id to the first match, matching [`Self::get`].
    fn parent_index(&self, i: usize) -> Option<usize> {
        let parent = self.nodes[i].parent.as_ref()?;
        self.index_of(parent)
    }

    /// Languages with no parent, or whose parent is absent from this assembly,
    /// in stored order. A dangling-parent node is a root of what was loaded.
    pub fn roots(&self) -> impl Iterator<Item = &LanguageGenome> + '_ {
        self.nodes.iter().filter(move |g| match &g.parent {
            None => true,
            Some(p) => self.index_of(p).is_none(),
        })
    }

    /// The children of `id`, in stored order. Linear scan; no index, no map.
    pub fn children<'a>(
        &'a self,
        id: &'a LanguageId,
    ) -> impl Iterator<Item = &'a LanguageGenome> + 'a {
        self.nodes
            .iter()
            .filter(move |g| g.parent.as_ref() == Some(id))
    }

    /// One edge per node whose parent is present, in stored (child) order.
    pub fn edges(&self) -> impl Iterator<Item = LineageEdge<'_>> + '_ {
        self.nodes.iter().enumerate().filter_map(move |(i, child)| {
            self.parent_index(i).map(|pi| LineageEdge {
                parent: &self.nodes[pi],
                child,
            })
        })
    }

    /// The ancestor chain from `id` toward its root, `id` first. Cycle-guarded
    /// **by node index**, not by id: with duplicate ids present (a family Error
    /// the walk must survive to report), an id-keyed guard could terminate a
    /// legitimate walk early. Stops before revisiting a node.
    pub fn ancestry(&self, id: &LanguageId) -> Vec<&LanguageGenome> {
        let mut chain = Vec::new();
        let mut seen = Vec::new();
        let mut cursor = self.index_of(id);
        while let Some(i) = cursor {
            if seen.contains(&i) {
                break; // a cycle — reported separately; the walk just terminates
            }
            seen.push(i);
            chain.push(&self.nodes[i]);
            cursor = self.parent_index(i);
        }
        chain
    }

    /// The transitive descendants of node `root_index` (parent-link closure),
    /// as stored indices **in node order**. Excludes the root itself.
    /// Cycle-guarded by visited index.
    ///
    /// The traversal below is a stack (depth-first), so its discovery order is
    /// not node order — and the closure order **does** reach output: it becomes
    /// the language list in each `CognateCoverage` gap, which `render_family`
    /// prints. So the result is sorted into stored (node) order before it
    /// returns, honouring the "languages in node order" contract on
    /// [`CognateCoverage::gaps`] regardless of tree shape.
    fn descendant_indices(&self, root_index: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut frontier = vec![root_index];
        let mut seen = vec![root_index];
        while let Some(i) = frontier.pop() {
            for (j, node) in self.nodes.iter().enumerate() {
                if node.parent.as_ref() == Some(&self.nodes[i].id) && !seen.contains(&j) {
                    seen.push(j);
                    out.push(j);
                    frontier.push(j);
                }
            }
        }
        out.sort_unstable();
        out
    }

    /// The distinct node indices that lie on a `parent`-link cycle. Empty when
    /// the family is acyclic. Deterministic (stored order).
    fn cycle_members(&self) -> Vec<usize> {
        let mut on_cycle = Vec::new();
        for start in 0..self.nodes.len() {
            // Walk up from `start`; if the walk returns to `start`, it is on a
            // cycle. Guarded so a walk into someone else's cycle terminates.
            let mut cursor = self.parent_index(start);
            let mut seen = vec![start];
            while let Some(i) = cursor {
                if i == start {
                    if !on_cycle.contains(&start) {
                        on_cycle.push(start);
                    }
                    break;
                }
                if seen.contains(&i) {
                    break;
                }
                seen.push(i);
                cursor = self.parent_index(i);
            }
        }
        on_cycle.sort_unstable();
        on_cycle
    }

    /// The distinct cognate sets a lexicon carries, in the lexicon's authored
    /// order (first occurrence).
    fn sets_of(genome: &LanguageGenome) -> Vec<CognateSetId> {
        let mut out: Vec<CognateSetId> = Vec::new();
        for entry in genome.lexicon.iter() {
            if !out.contains(&entry.cognate_set) {
                out.push(entry.cognate_set.clone());
            }
        }
        out
    }

    fn has_set(genome: &LanguageGenome, set: &CognateSetId) -> bool {
        genome.lexicon.iter().any(|e| &e.cognate_set == set)
    }

    fn has_word(genome: &LanguageGenome, id: &WordId) -> bool {
        genome.lexicon.get(id).is_some()
    }

    /// Coverage of each root's cognate sets across its transitive descendants.
    /// Roots with zero descendants are omitted (nothing to compare). Roots in
    /// node order; sets in the root lexicon's authored order; languages in node
    /// order. Never sorted.
    pub fn cognate_coverage(&self) -> Vec<CognateCoverage> {
        let mut coverage = Vec::new();
        for (i, root) in self.nodes.iter().enumerate() {
            // A "root" here is any node with descendants — the coverage question
            // is meaningful for every internal node, but the acceptance story is
            // about lineage roots. We report for genuine roots (no present
            // parent) with ≥1 descendant.
            let is_root = match &root.parent {
                None => true,
                Some(p) => self.index_of(p).is_none(),
            };
            if !is_root {
                continue;
            }
            let descendants = self.descendant_indices(i);
            if descendants.is_empty() {
                continue;
            }
            let sets = Self::sets_of(root);
            let mut universal = 0usize;
            let mut gaps: Vec<(CognateSetId, Vec<LanguageId>)> = Vec::new();
            for set in &sets {
                let lacking: Vec<LanguageId> = descendants
                    .iter()
                    .filter(|&&d| !Self::has_set(&self.nodes[d], set))
                    .map(|&d| self.nodes[d].id.clone())
                    .collect();
                if lacking.is_empty() {
                    universal += 1;
                } else {
                    gaps.push((set.clone(), lacking));
                }
            }
            coverage.push(CognateCoverage {
                root: root.id.clone(),
                descendants: descendants.len(),
                sets: sets.len(),
                universal,
                gaps,
            });
        }
        coverage
    }

    /// Family-level validation (`docs/adr/0008`, spec §7). Absorbs each node's
    /// own [`LanguageGenome::validate`] under its language id as scope, then adds
    /// the cross-file checks no single file can run. Severities per §17: only
    /// structural breakage — the things that make graph walks, joins, or
    /// timelines meaningless — errors.
    pub fn validate_family(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Each member's own report, namespaced by its language id so
        // `coastal.lexicon.syllable_shape_mismatch` stays legible.
        for node in &self.nodes {
            report.absorb(node.id.as_str(), node.validate());
        }

        // family.duplicate_language_id (Error) — reported once per duplicated id,
        // in stored order of the first repeat. ADR-0003: duplicate-id detection
        // is an Error everywhere.
        let mut seen_ids: Vec<&LanguageId> = Vec::new();
        let mut reported: Vec<&LanguageId> = Vec::new();
        for node in &self.nodes {
            if seen_ids.contains(&&node.id) {
                if !reported.contains(&&node.id) {
                    report.push(
                        Issue::new(
                            Severity::Error,
                            "family.duplicate_language_id",
                            format!(
                                "two languages in this family share the id `{}`; a lineage \
                                 graph keyed on identity cannot tell them apart",
                                node.id
                            ),
                        )
                        .about(&node.id),
                    );
                    reported.push(&node.id);
                }
            } else {
                seen_ids.push(&node.id);
            }
        }

        // family.parent_cycle (Error) — one per cycle member, naming it. The
        // cross-file extension of the genome's `self_parent`. `ancestry` is
        // index-guarded, so every walk terminates regardless.
        for &i in &self.cycle_members() {
            report.push(
                Issue::new(
                    Severity::Error,
                    "family.parent_cycle",
                    format!(
                        "language `{}` lies on a parent cycle; following its ancestry never \
                         reaches a proto-language",
                        self.nodes[i].id
                    ),
                )
                .about(&self.nodes[i].id),
            );
        }

        // Per-edge checks: depth regression, cognate gaps, word-id orphans,
        // no-divergence. In stored (child) order.
        for edge in self.edges() {
            let child = edge.child;
            let parent = edge.parent;

            if edge.elapsed_years() < 0 {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "family.depth_regression",
                        format!(
                            "language `{}` is dated {} years but its parent `{}` is at {}; \
                             time runs forwards, and a negative edge makes every timeline \
                             label meaningless",
                            child.id,
                            child.lineage_depth_years,
                            parent.id,
                            parent.lineage_depth_years
                        ),
                    )
                    .about(&child.id),
                );
            }

            // Cognate gaps, both directions. A set in the parent with no reflex
            // in the child is word death — real history (M7 produces it), a
            // Warning naming where. A set in the child its parent lacks is legal
            // innovation, but paired with a `_missing` it is what a re-minted or
            // typo'd cognate id looks like, so both sides are named (§8.6's
            // failure mode, surfaced without policing).
            for set in Self::sets_of(parent) {
                if !Self::has_set(child, &set) {
                    report.push(
                        Issue::new(
                            Severity::Warning,
                            "family.cognate_set_missing",
                            format!(
                                "cognate set `{set}` is in `{}` but has no reflex in its \
                                 daughter `{}`; the comparative table will show a gap here",
                                parent.id, child.id
                            ),
                        )
                        .about(&child.id),
                    );
                }
            }
            for set in Self::sets_of(child) {
                if !Self::has_set(parent, &set) {
                    report.push(
                        Issue::new(
                            Severity::Warning,
                            "family.cognate_set_unrooted",
                            format!(
                                "daughter `{}` carries cognate set `{set}`, which its parent \
                                 `{}` lacks; legal if innovated, but a re-minted or \
                                 mistyped id looks exactly like this",
                                child.id, parent.id
                            ),
                        )
                        .about(&child.id),
                    );
                }
            }

            // family.word_id_orphan (Warning) — the checkable shadow of the
            // derivable-ancestor invariant (§2.4) over hand-edited files.
            for word in child.lexicon.iter() {
                if !Self::has_word(parent, &word.id) {
                    report.push(
                        Issue::new(
                            Severity::Warning,
                            "family.word_id_orphan",
                            format!(
                                "word `{}` of `{}` has no same-id ancestor in its parent \
                                 `{}`; fork and evolve copy word ids verbatim, so a daughter \
                                 word id should always resolve in the parent",
                                word.id, child.id, parent.id
                            ),
                        )
                        .about(&child.id),
                    );
                }
            }

            // family.no_divergence (Note) — freshly forked, no history of its
            // own yet. Works at any parent depth, unlike the genome-level
            // `no_elapsed_time` (§2.2).
            if child.applied_rules == parent.applied_rules {
                report.push(
                    Issue::new(
                        Severity::Note,
                        "family.no_divergence",
                        format!(
                            "language `{}` has the same rule history as its parent `{}`; \
                             apply a rule history to make it its own language",
                            child.id, parent.id
                        ),
                    )
                    .about(&child.id),
                );
            }
        }

        // family.dangling_parent (Warning) — a node names a parent not in the
        // assembly. Not an Error: inspecting a subtree is legitimate; the node
        // renders as a root. In stored order.
        for node in &self.nodes {
            if let Some(parent) = &node.parent
                && self.index_of(parent).is_none()
            {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "family.dangling_parent",
                        format!(
                            "language `{}` names parent `{parent}`, which was not loaded; \
                             pass its file too, or it renders as a root",
                            node.id
                        ),
                    )
                    .about(&node.id),
                );
            }
        }

        // family.multiple_roots (Note) — fine for comparison; noted because
        // coverage is computed per root.
        let root_count = self.roots().count();
        if root_count > 1 {
            report.note(
                "family.multiple_roots",
                format!("this family has {root_count} roots; each is covered separately below"),
            );
        }

        report
    }
}

/// Renders the family tree and cognate-coverage summary as terminal text.
///
/// Infallible — nothing in it can fail, so no `Result` theatre. In the library,
/// not the CLI, for the `render_derivation` precedent: the M11 UI must render
/// the identical text through the identical function. The validation report is
/// **not** part of this string; the CLI prints it separately, so the snapshot
/// test (pinned for M6) covers only tree + coverage.
///
/// Walks from `roots()` in stored order, children in stored order, each node
/// rendered at most once (visited by node index). Any node never reached — cycle
/// members, second occurrences of a duplicated id — is listed in an explicit
/// trailing `unrooted or duplicated:` section, so no member is ever silently
/// absent from the view the report describes.
pub fn render_family(graph: &LineageGraph) -> String {
    let mut out = String::new();
    let mut visited = vec![false; graph.len()];

    // Roots, in stored order. A root's own label is "proto" when it truly has no
    // parent, else its total depth (a dangling-parent orphan).
    let root_indices: Vec<usize> = graph
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, g)| match &g.parent {
            None => true,
            Some(p) => graph.index_of(p).is_none(),
        })
        .map(|(i, _)| i)
        .collect();

    for &r in &root_indices {
        render_subtree(graph, r, "", true, true, &mut visited, &mut out);
    }

    // Coverage summary.
    let coverage = graph.cognate_coverage();
    if !coverage.is_empty() {
        out.push('\n');
        for cov in &coverage {
            out.push_str(&format!(
                "cognate coverage — {}: {} set{}, {} descendant{}, {}/{} present in all\n",
                cov.root,
                cov.sets,
                if cov.sets == 1 { "" } else { "s" },
                cov.descendants,
                if cov.descendants == 1 { "" } else { "s" },
                cov.universal,
                cov.sets,
            ));
            for (set, lacking) in &cov.gaps {
                let names: Vec<String> = lacking.iter().map(|l| l.to_string()).collect();
                out.push_str(&format!(
                    "    gap: {set} absent from {}\n",
                    names.join(", ")
                ));
            }
        }
    }

    // Any node never reached: cycle members and duplicate-id shadows.
    let leftovers: Vec<usize> = (0..graph.len()).filter(|&i| !visited[i]).collect();
    if !leftovers.is_empty() {
        out.push_str("\nunrooted or duplicated:\n");
        for i in leftovers {
            let g = &graph.nodes()[i];
            out.push_str(&format!("  {} ({})\n", g.name, g.id));
        }
    }

    out
}

/// Renders node `i` and its subtree. `prefix` is the running indentation for
/// this node's *children*; `is_root` suppresses the connector; `is_last`
/// selects the connector glyph.
fn render_subtree(
    graph: &LineageGraph,
    i: usize,
    prefix: &str,
    is_root: bool,
    is_last: bool,
    visited: &mut [bool],
    out: &mut String,
) {
    if visited[i] {
        return; // a node reachable twice (duplicate id) is drawn once
    }
    visited[i] = true;
    let g = &graph.nodes()[i];

    let connector = if is_root {
        ""
    } else if is_last {
        "└─ "
    } else {
        "├─ "
    };

    // The depth label: "proto" for a true root, "+Ny" as the edge delta from the
    // rendered parent otherwise. A dangling-parent orphan reads as a root but is
    // not proto, so it shows its own total depth.
    let depth_label = if is_root {
        match &g.parent {
            None => "proto".to_owned(),
            Some(_) => format!("@{}y", g.lineage_depth_years),
        }
    } else {
        // The parent is the rendered ancestor; delta is our depth minus theirs.
        match graph.parent_index(i) {
            Some(pi) => format!(
                "+{}y",
                g.lineage_depth_years - graph.nodes()[pi].lineage_depth_years
            ),
            None => format!("@{}y", g.lineage_depth_years),
        }
    };

    out.push_str(&format!(
        "{prefix}{connector}{} ({}) — {} · {} phonemes · {} word{} · {} rule{}\n",
        g.name,
        g.id,
        depth_label,
        g.phonemes.len(),
        g.lexicon.len(),
        if g.lexicon.len() == 1 { "" } else { "s" },
        g.applied_rules.len(),
        if g.applied_rules.len() == 1 { "" } else { "s" },
    ));

    // Children in stored order. Their indentation continues this node's prefix.
    let child_indices: Vec<usize> = graph
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.parent.as_ref() == Some(&g.id))
        .map(|(j, _)| j)
        .collect();

    let child_prefix = if is_root {
        String::new()
    } else if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}│  ")
    };

    let last = child_indices.len().saturating_sub(1);
    for (k, &c) in child_indices.iter().enumerate() {
        render_subtree(graph, c, &child_prefix, false, k == last, visited, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_core::{CognateSetId, PhonemeId, WordId};
    use stem_lexicon::{Lexicon, PartOfSpeech, WordEntry, WordSource};
    use stem_phonology::{Phoneme, PhonemeInventory, Root, SegmentKind, Syllable};

    fn featured(id: &str) -> Phoneme {
        // Every check in this module reads ids, depths, lexicons — never feature
        // bundles — so a minimal but *validly featured* vowel keeps
        // `features_unspecified` (an Error since M3) from tripping the absorbed
        // per-node reports in the tests that assert `is_ok`.
        let tokens = [
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
        ];
        let bundle = stem_phonology::FeatureBundle::try_from(
            tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        )
        .expect("valid vowel");
        Phoneme::new(id, "a", SegmentKind::Vowel).with_features(bundle)
    }

    /// A genome with `id`, a parent, a depth, and a lexicon of the given cognate
    /// sets (each set gets one word, word id `w_{ordinal}`). Enough for every
    /// family-level check.
    fn lang(id: &str, parent: Option<&str>, depth: i32, sets: &[&str]) -> LanguageGenome {
        let mut genome = LanguageGenome::proto(id, id.to_uppercase());
        genome.parent = parent.map(Into::into);
        genome.lineage_depth_years = depth;
        genome.phonemes = PhonemeInventory::from_phonemes([featured("ph_a")]);
        let entries = sets.iter().enumerate().map(|(i, set)| WordEntry {
            id: WordId::sequential(i + 1),
            concept: None,
            phonemic_form: Root {
                syllables: vec![Syllable {
                    pattern: "V".to_owned(),
                    segments: vec![PhonemeId::new("ph_a")],
                    stress: None,
                }],
            },
            glosses: vec!["x".to_owned()],
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: CognateSetId::new(*set),
            source: WordSource::Authored,
            trace: None,
        });
        genome.lexicon = Lexicon::from_entries(entries);
        genome
    }

    /// The three-daughter shape of the acceptance family, structurally.
    fn family() -> LineageGraph {
        LineageGraph::assemble(vec![
            lang("proto", None, 0, &["cog_0001", "cog_0002"]),
            lang("coastal", Some("proto"), 470, &["cog_0001", "cog_0002"]),
            lang("highland", Some("proto"), 460, &["cog_0001", "cog_0002"]),
            lang("riverine", Some("proto"), 420, &["cog_0001", "cog_0002"]),
        ])
    }

    #[test]
    fn a_lineage_graph_derives_edges_from_parent_fields_alone() {
        let graph = family();
        let edges: Vec<_> = graph
            .edges()
            .map(|e| {
                (
                    e.parent.id.as_str().to_owned(),
                    e.child.id.as_str().to_owned(),
                )
            })
            .collect();
        assert_eq!(
            edges,
            vec![
                ("proto".to_owned(), "coastal".to_owned()),
                ("proto".to_owned(), "highland".to_owned()),
                ("proto".to_owned(), "riverine".to_owned()),
            ],
            "edges are exactly the present parent links, in child (stored) order"
        );
    }

    #[test]
    fn graph_walks_follow_given_order_never_map_order() {
        let graph = family();
        let children: Vec<_> = graph
            .children(&"proto".into())
            .map(|c| c.id.as_str().to_owned())
            .collect();
        assert_eq!(children, vec!["coastal", "highland", "riverine"]);
        assert_eq!(graph.roots().count(), 1);
    }

    #[test]
    fn an_edge_reports_elapsed_years_as_the_depth_delta() {
        let graph = LineageGraph::assemble(vec![
            lang("proto", None, 200, &["cog_0001"]),
            lang("daughter", Some("proto"), 470, &["cog_0001"]),
        ]);
        let edge = graph.edges().next().unwrap();
        assert_eq!(
            edge.elapsed_years(),
            270,
            "elapsed is child depth minus parent depth, not the child's total"
        );
    }

    #[test]
    fn a_family_with_a_duplicate_language_id_errors() {
        let graph = LineageGraph::assemble(vec![
            lang("proto", None, 0, &["cog_0001"]),
            lang("coastal", Some("proto"), 100, &["cog_0001"]),
            lang("coastal", Some("proto"), 100, &["cog_0001"]),
        ]);
        let report = graph.validate_family();
        assert!(
            report
                .errors()
                .any(|i| i.code == "family.duplicate_language_id"),
            "{report}"
        );
    }

    #[test]
    fn ancestry_terminates_on_a_parent_cycle_and_the_family_reports_it() {
        // proto -> a -> b -> a  (a and b cycle)
        let graph = LineageGraph::assemble(vec![
            lang("a", Some("b"), 100, &["cog_0001"]),
            lang("b", Some("a"), 100, &["cog_0001"]),
        ]);
        // The walk must not hang.
        let chain = graph.ancestry(&"a".into());
        assert!(chain.len() <= 2, "the walk terminates: {}", chain.len());
        let report = graph.validate_family();
        assert!(
            report.errors().any(|i| i.code == "family.parent_cycle"),
            "{report}"
        );
    }

    #[test]
    fn a_child_older_than_its_parent_is_a_depth_regression_error() {
        let graph = LineageGraph::assemble(vec![
            lang("proto", None, 500, &["cog_0001"]),
            lang("daughter", Some("proto"), 400, &["cog_0001"]),
        ]);
        let report = graph.validate_family();
        assert!(
            report.errors().any(|i| i.code == "family.depth_regression"),
            "{report}"
        );
    }

    #[test]
    fn a_dangling_parent_warns_and_the_node_renders_as_a_root() {
        let graph = LineageGraph::assemble(vec![lang(
            "daughter",
            Some("absent_proto"),
            100,
            &["cog_0001"],
        )]);
        let report = graph.validate_family();
        assert!(
            report
                .warnings()
                .any(|i| i.code == "family.dangling_parent"),
            "{report}"
        );
        assert_eq!(
            graph.roots().count(),
            1,
            "the orphan is a root of what loaded"
        );
    }

    #[test]
    fn a_daughter_missing_a_cognate_set_warns_as_a_gap_not_an_error() {
        let graph = LineageGraph::assemble(vec![
            lang("proto", None, 0, &["cog_0001", "cog_0002"]),
            lang("daughter", Some("proto"), 100, &["cog_0001"]), // lost cog_0002
        ]);
        let report = graph.validate_family();
        assert!(
            report
                .warnings()
                .any(|i| i.code == "family.cognate_set_missing"),
            "{report}"
        );
        assert!(
            report.is_ok(),
            "a lost set is history, not an error: {report}"
        );
    }

    #[test]
    fn an_unrooted_cognate_set_warns_and_pairs_with_its_gap() {
        let graph = LineageGraph::assemble(vec![
            lang("proto", None, 0, &["cog_0001"]),
            lang("daughter", Some("proto"), 100, &["cog_0002"]), // swapped id
        ]);
        let report = graph.validate_family();
        assert!(
            report
                .warnings()
                .any(|i| i.code == "family.cognate_set_unrooted"),
            "{report}"
        );
        assert!(
            report
                .warnings()
                .any(|i| i.code == "family.cognate_set_missing"),
            "the missing side is named too, so a re-mint is visible from both sides: {report}"
        );
    }

    #[test]
    fn a_child_word_id_absent_from_the_parent_warns_as_an_orphan() {
        let mut daughter = lang("daughter", Some("proto"), 100, &["cog_0001"]);
        // Renumber the daughter's only word so it has no same-id ancestor.
        daughter.lexicon = Lexicon::from_entries(daughter.lexicon.iter().map(|e| {
            let mut e = e.clone();
            e.id = WordId::new("w_9999");
            e
        }));
        let graph = LineageGraph::assemble(vec![lang("proto", None, 0, &["cog_0001"]), daughter]);
        let report = graph.validate_family();
        assert!(
            report.warnings().any(|i| i.code == "family.word_id_orphan"),
            "{report}"
        );
    }

    #[test]
    fn a_freshly_forked_daughter_notes_no_divergence_at_any_parent_depth() {
        // Parent at depth 300 (not a proto), daughter copied its rule history.
        let graph = LineageGraph::assemble(vec![
            lang("mid", Some("absent"), 300, &["cog_0001"]),
            lang("fresh", Some("mid"), 340, &["cog_0001"]),
        ]);
        let report = graph.validate_family();
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "family.no_divergence"),
            "no_divergence must fire for a deep parent too, unlike no_elapsed_time: {report}"
        );
    }

    #[test]
    fn cognate_coverage_counts_sets_present_in_every_descendant() {
        let coverage = family().cognate_coverage();
        assert_eq!(coverage.len(), 1, "one root with descendants");
        let cov = &coverage[0];
        assert_eq!(cov.descendants, 3);
        assert_eq!(cov.sets, 2);
        assert_eq!(cov.universal, 2, "both sets present in all three daughters");
        assert!(cov.gaps.is_empty(), "{:?}", cov.gaps);
    }

    /// A gap's language list must be in **node (stored) order**, whatever the
    /// tree shape — the closure walk is depth-first, so a multi-level tree is
    /// where DFS order and node order diverge, and this list reaches the
    /// `stemma family` gap render. Regression for the M4 review finding.
    #[test]
    fn a_coverage_gap_lists_the_lacking_languages_in_node_order() {
        // proto(0) -> a(1) -> b(2), and proto(0) -> c(3). cog_gap is in proto and
        // a, but absent from b and c. DFS discovery order of the descendants is
        // [a, c, b] = [1, 3, 2]; node order is [a, b, c] = [1, 2, 3].
        let graph = LineageGraph::assemble(vec![
            lang("proto", None, 0, &["cog_keep", "cog_gap"]),
            lang("a", Some("proto"), 100, &["cog_keep", "cog_gap"]),
            lang("b", Some("a"), 200, &["cog_keep"]),
            lang("c", Some("proto"), 100, &["cog_keep"]),
        ]);
        let coverage = graph.cognate_coverage();
        let gap = coverage[0]
            .gaps
            .iter()
            .find(|(set, _)| set.as_str() == "cog_gap")
            .expect("cog_gap is a gap");
        let lacking: Vec<&str> = gap.1.iter().map(|l| l.as_str()).collect();
        assert_eq!(
            lacking,
            vec!["b", "c"],
            "the lacking list must be in node order, not DFS discovery order"
        );
    }

    #[test]
    fn coverage_omits_roots_with_no_descendants() {
        let graph = LineageGraph::assemble(vec![lang("lonely", None, 0, &["cog_0001"])]);
        assert!(
            graph.cognate_coverage().is_empty(),
            "a childless root has nothing to compare"
        );
    }

    #[test]
    fn a_member_genomes_own_issues_reach_the_family_report_namespaced() {
        let mut broken = lang("broken", None, 0, &["cog_0001"]);
        broken.phonemes = PhonemeInventory::new(); // empty inventory: an Error
        let graph = LineageGraph::assemble(vec![broken]);
        let report = graph.validate_family();
        assert!(
            report.errors().any(|i| i.code == "broken.phonology.empty"),
            "a member's own error must reach the family report under its id scope: {report}"
        );
    }

    #[test]
    fn render_family_lists_cycle_members_under_an_explicit_trailer() {
        let graph = LineageGraph::assemble(vec![
            lang("a", Some("b"), 100, &["cog_0001"]),
            lang("b", Some("a"), 100, &["cog_0001"]),
        ]);
        let text = render_family(&graph);
        assert!(
            text.contains("unrooted or duplicated:"),
            "cycle members are never silently absent from the view: {text}"
        );
    }

    #[test]
    fn render_family_is_a_pure_function_of_the_given_node_order() {
        let a = render_family(&family());
        let b = render_family(&family());
        assert_eq!(a, b, "same node order, identical bytes");
    }

    /// The cognate-mint invariant, defended in `stem_genome` the way
    /// `stem_lexicon` defends it in itself: by reading this crate's own sources.
    /// `include_str!` cannot cross a crate boundary, so each crate that could
    /// mint carries its own scan (`docs/adr/0008`). This one enumerates
    /// `src/*.rs` at **runtime** via `CARGO_MANIFEST_DIR` — no hand-maintained
    /// file list to fall out of date the next time a module is added — and
    /// scans only the non-test region of each file (M4's own unit tests
    /// legitimately construct entries with a `CognateSetId`).
    #[test]
    fn stem_genome_never_mints_a_cognate_set() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .expect("stem_genome/src is readable")
            .map(|e| e.expect("dir entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .collect();
        files.sort();
        assert!(
            files.len() >= 3,
            "expected to find the crate's modules, saw {files:?}"
        );

        for path in files {
            let src = std::fs::read_to_string(&path).expect("readable source");
            for (n, line) in src.lines().enumerate() {
                // Stop at the test module: helpers there build entries and are
                // not production mint sites.
                let trimmed = line.trim_start();
                if trimmed.starts_with("#[cfg(test)]") || trimmed == "mod tests {" {
                    break;
                }
                if trimmed.starts_with("//") {
                    continue;
                }
                let mints =
                    line.contains("CognateSetId::new") || line.contains("scoped_cognate_set");
                assert!(
                    !mints,
                    "{}:{} mints a CognateSetId; forking must copy cognate sets verbatim, \
                     never mint (only stem_lexicon::scoped_cognate_set may)",
                    path.file_name().unwrap().to_string_lossy(),
                    n + 1
                );
            }
        }
    }
}
