//! The explorer's state and its views.
//!
//! # The rule this crate exists under
//!
//! **The UI holds no logic.** Every panel below is a *presentation* of a string a
//! library crate produced — `render_word_history`, `render_cognate_table`,
//! `render_family`, `render_profile`, `render_paradigm`. This crate computes no
//! form, resolves no meaning, and applies no rule.
//!
//! That is not a style preference; it has been the plan since M4, when
//! `render_family` was moved into `stem_genome` with the note *"the M11 UI must
//! render the identical text through the identical function"*. The payoff is that
//! the CLI and this window cannot disagree about what a word's history is — there is
//! one renderer, and a bug in it shows up in both. A UI that re-derived any of this
//! for prettier display would be a second engine with a second set of answers.
//!
//! # Read-only, deliberately
//!
//! §20.5 names UI complexity as the risk and prescribes the mitigation: *keep views
//! read-only before adding full editing.* Nothing here writes a file. Opening,
//! looking, and tracing is the whole of M11.

use std::path::{Path, PathBuf};

use stem_core::Validate;
use stem_genome::{LanguageGenome, LineageGraph};

/// Which view the main panel is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// §10.2's "Trace This Word" — the killer feature.
    Trace,
    /// §10.3's comparative table: the daughters side by side.
    Cognates,
    /// The family tree and its cognate coverage.
    Family,
    /// §17's plausibility profile plus the graded report.
    Profile,
}

impl View {
    pub const ALL: [View; 4] = [View::Trace, View::Cognates, View::Family, View::Profile];

    pub fn label(self) -> &'static str {
        match self {
            View::Trace => "Trace a word",
            View::Cognates => "Cognate table",
            View::Family => "Family",
            View::Profile => "Profile",
        }
    }
}

pub struct App {
    /// Every language the user has opened, in the order they opened them. The
    /// first is the reference the cognate table resolves meanings against —
    /// `cognate_table`'s own contract, surfaced rather than hidden.
    languages: Vec<LanguageGenome>,
    /// Index into `languages` of the one being inspected.
    selected: usize,
    /// The word whose history the Trace view is showing, by id.
    traced: Option<String>,
    /// Meanings the cognate table is showing, one per line as typed.
    meanings: String,
    view: View,
    /// The last thing that went wrong, shown in a banner until dismissed.
    error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            languages: Vec::new(),
            selected: 0,
            traced: None,
            meanings: "star\nsun\nwater".to_owned(),
            view: View::Trace,
            error: None,
        }
    }
}

impl App {
    /// Opens the paths given on the command line, so `stemma-ui a.ron b.ron` works
    /// like every other tool in this project.
    pub fn with_paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut app = Self::default();
        for path in paths {
            app.open(&path);
        }
        app
    }

    /// Loads one language. Failure is reported in the banner, never a panic — a
    /// user who picks the wrong file should be told, not crashed.
    fn open(&mut self, path: &Path) {
        match stem_io::load::<LanguageGenome>(path) {
            Ok(genome) => {
                self.languages.push(genome);
                self.selected = self.languages.len() - 1;
                self.traced = None;
            }
            Err(e) => self.error = Some(format!("could not open `{}`: {e}", path.display())),
        }
    }

    fn current(&self) -> Option<&LanguageGenome> {
        self.languages.get(self.selected)
    }

    /// The graph over everything open, in the order opened. Derived on demand and
    /// never stored — `docs/adr/0008`'s rule, which this crate has no reason to
    /// break just because it is redrawing every frame.
    fn graph(&self) -> LineageGraph {
        LineageGraph::assemble(self.languages.clone())
    }
}

impl eframe::App for App {
    // egui 0.36 hands the app a `Ui` rather than a `Context`, and panels nest inside
    // it — so the whole window is built from one root `Ui` downward.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Files dropped onto the window open like any other — the ordinary desktop
        // gesture, and free from the framework.
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        for path in dropped {
            self.open(&path);
        }

        self.top_bar(ui);
        self.side_panel(ui);
        self.main_panel(ui);
    }
}

impl App {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::containers::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Stemma");
                ui.separator();
                if ui.button("Open…").clicked() {
                    let picked = rfd::FileDialog::new()
                        .add_filter("Stemma language", &["ron", "json"])
                        .set_title("Open a language")
                        .pick_files();
                    for path in picked.into_iter().flatten() {
                        self.open(&path);
                    }
                }
                if ui.button("Close all").clicked() {
                    self.languages.clear();
                    self.selected = 0;
                    self.traced = None;
                }
                ui.separator();
                for view in View::ALL {
                    ui.selectable_value(&mut self.view, view, view.label());
                }
            });
        });

        if let Some(message) = self.error.clone() {
            egui::containers::Panel::top("error").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(0xC0, 0x39, 0x2B), "⚠");
                    ui.label(message);
                    if ui.button("dismiss").clicked() {
                        self.error = None;
                    }
                });
            });
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::containers::Panel::left("languages")
            .default_size(240.0)
            .show(ui, |ui| {
                ui.heading("Languages");
                ui.label(
                    egui::RichText::new("opened order — the first is the table's reference")
                        .small()
                        .weak(),
                );
                ui.separator();

                if self.languages.is_empty() {
                    ui.label("Nothing open.");
                    ui.label(
                        egui::RichText::new("Use Open…, or drop a .ron file on this window.")
                            .small()
                            .weak(),
                    );
                    return;
                }

                for i in 0..self.languages.len() {
                    let genome = &self.languages[i];
                    let label = format!("{} ({})", genome.name, genome.id);
                    if ui.selectable_label(self.selected == i, label).clicked() {
                        self.selected = i;
                        self.traced = None;
                    }
                }

                ui.separator();
                // The word list doubles as the trace picker — §10.2's "clicking any
                // word should open a full trace", literally.
                if let Some(genome) = self.current() {
                    ui.heading("Words");
                    let inventory = &genome.phonemes;
                    let rows: Vec<(String, String, String)> = genome
                        .lexicon
                        .iter()
                        .map(|e| {
                            (
                                e.id.to_string(),
                                e.written(inventory).unwrap_or_else(|_| "?".to_owned()),
                                e.display_gloss().unwrap_or("?").to_owned(),
                            )
                        })
                        .collect();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (id, form, gloss) in rows {
                            let selected = self.traced.as_deref() == Some(id.as_str());
                            let label = format!("{form}  — {gloss}");
                            if ui.selectable_label(selected, label).clicked() {
                                self.traced = Some(id);
                                self.view = View::Trace;
                            }
                        }
                    });
                }
            });
    }

    fn main_panel(&mut self, ui: &mut egui::Ui) {
        egui::containers::CentralPanel::default().show(ui, |ui| {
            if self.languages.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.heading("Open a language to begin");
                    ui.label("File → Open…, or drop a .ron or .json file onto this window.");
                });
                return;
            }
            match self.view {
                View::Trace => self.trace_view(ui),
                View::Cognates => self.cognates_view(ui),
                View::Family => self.family_view(ui),
                View::Profile => self.profile_view(ui),
            }
        });
    }

    /// §10.2's killer feature. The text is `render_word_history`'s, verbatim — the
    /// same bytes `stemma trace` prints.
    fn trace_view(&mut self, ui: &mut egui::Ui) {
        let Some(genome) = self.current() else { return };

        let Some(id) = self.traced.clone() else {
            ui.heading("Trace a word");
            ui.label("Pick a word from the list on the left.");
            return;
        };
        let Some(entry) = genome.lexicon.iter().find(|e| e.id.as_str() == id) else {
            ui.label(format!("`{id}` is not in this language."));
            return;
        };

        match stem_genome::render_word_history(genome, entry) {
            Ok(text) => {
                ui.heading(format!(
                    "{} — {}",
                    entry.display_gloss().unwrap_or("?"),
                    genome.name
                ));
                ui.label(
                    egui::RichText::new(
                        "the same text `stemma trace` prints — one renderer, one answer",
                    )
                    .small()
                    .weak(),
                );
                ui.separator();
                monospace_scroll(ui, &text);
            }
            Err(e) => {
                ui.colored_label(egui::Color32::from_rgb(0xC0, 0x39, 0x2B), e.to_string());
            }
        }
    }

    /// §10.3, and the "view daughters side by side" half of M11's acceptance.
    fn cognates_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Cognate table");
        ui.label(
            egui::RichText::new(
                "joined by shared ancestry, not by meaning — a reflex keeps its row \
                 even after its sense drifts",
            )
            .small()
            .weak(),
        );
        ui.horizontal(|ui| {
            ui.label("Meanings (one per line):");
            ui.add(
                egui::TextEdit::multiline(&mut self.meanings)
                    .desired_rows(3)
                    .desired_width(220.0),
            );
        });
        ui.separator();

        if self.languages.len() < 2 {
            ui.label("Open at least two languages to compare them.");
            return;
        }

        let meanings: Vec<String> = self
            .meanings
            .lines()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(str::to_owned)
            .collect();
        if meanings.is_empty() {
            ui.label("Type a meaning above, e.g. `star`.");
            return;
        }

        match self.graph().cognate_table(&meanings) {
            Ok(table) => {
                monospace_scroll(ui, &stem_genome::render_cognate_table(&table));
                for note in &table.notes {
                    ui.label(egui::RichText::new(format!("· {note}")).small().weak());
                }
            }
            Err(e) => {
                ui.colored_label(egui::Color32::from_rgb(0xC0, 0x39, 0x2B), e.to_string());
            }
        }
    }

    fn family_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Family");
        ui.separator();
        let graph = self.graph();
        monospace_scroll(ui, &stem_genome::render_family(&graph));
    }

    fn profile_view(&mut self, ui: &mut egui::Ui) {
        let Some(genome) = self.current() else { return };
        ui.heading(format!("Profile — {}", genome.name));
        ui.separator();

        let mut text = stem_genome::render_profile(&genome.plausibility_profile(), &genome.name);
        // The graded report below the bands, exactly as `stemma profile` shows it.
        let report = genome.validate();
        text.push('\n');
        if report.issues.is_empty() {
            text.push_str("✓ no issues\n");
        } else {
            for issue in &report.issues {
                text.push_str(&format!("  {issue}\n"));
            }
        }
        monospace_scroll(ui, &text);
    }
}

/// Renders library text in a scrollable monospace block.
///
/// Monospace is not decoration: every one of these renderers aligns its columns by
/// **character count** for exactly this, and a proportional font would undo the work
/// `pad` does. `selectable` so a user can copy a derivation out.
fn monospace_scroll(ui: &mut egui::Ui, text: &str) {
    egui::ScrollArea::both().show(ui, |ui| {
        ui.add(
            egui::Label::new(egui::RichText::new(text).monospace())
                .selectable(true)
                .wrap_mode(egui::TextWrapMode::Extend),
        );
    });
}

#[cfg(test)]
mod guard {
    /// **The UI holds no logic** — enforced by reading the source, the same crude
    /// and honest discipline as the cognate-mint scan and the engine's
    /// morphology/semantics guards.
    ///
    /// This crate may *load*, *assemble* and *render*. It may not apply a rule,
    /// drift a meaning, compose a word or mint an id. The moment it does, the CLI
    /// and this window can disagree about what a word's history is — and since M4
    /// the whole plan has been that they cannot, because there is exactly one
    /// renderer behind both.
    #[test]
    fn the_ui_computes_nothing_it_could_instead_ask_a_library_for() {
        for (name, src) in [
            ("app.rs", include_str!("app.rs")),
            ("main.rs", include_str!("main.rs")),
        ] {
            for (n, line) in src.lines().enumerate() {
                // Stop at this module: it names the banned tokens itself.
                if line.contains("mod guard") {
                    break;
                }
                let code = line.split("//").next().unwrap_or("");
                for banned in [
                    "apply_rules",
                    "apply_drift",
                    "scoped_cognate_set",
                    "parse_rule_set",
                    "::compose",
                    "::inflect",
                    "stem_io::save",
                ] {
                    assert!(
                        !code.contains(banned),
                        "{name}:{} calls `{banned}` — the UI is a second front end onto the \
                         libraries, not a second engine, and it is read-only (§20.5)",
                        n + 1
                    );
                }
            }
        }
    }
}
