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
//! # Editing, under the same rule (M16)
//!
//! §20.5 said "keep views read-only **before** adding full editing", not "never", and
//! M16 is the after. The fence it protected is intact and is the reason editing was
//! worth waiting for: this crate still computes nothing.
//!
//! Every edit is one [`stem_genome::Edit`] value handed to `apply_edit` — the same
//! call `stemma set-gloss` makes, so a file saved from this window is byte-identical
//! to one saved from the command line. What the window contributes is a text box and
//! a button. The guard test below keeps the ban on engine entrypoints; only
//! `stem_io::save` left the list, and only behind an explicit **Save** press.
//!
//! **Undo is the file.** Nothing autosaves, nothing is written until Save, and there
//! is no in-memory history stack — the file on disk is a better undo than any of
//! those, and the user already knows how it works. Unsaved edits are marked, so
//! closing with changes pending is a visible choice rather than a silent loss.

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
    /// M16: change this language, through the same library the CLI uses.
    Edit,
}

impl View {
    pub const ALL: [View; 5] = [
        View::Trace,
        View::Cognates,
        View::Family,
        View::Profile,
        View::Edit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            View::Trace => "Trace a word",
            View::Cognates => "Cognate table",
            View::Family => "Family",
            View::Profile => "Profile",
            View::Edit => "Edit",
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

    // --- M16: editing ---
    /// Where each open language came from, so Save has somewhere to write. Parallel
    /// to `languages` by index — the same relationship the trace picker already has
    /// to the word list, and cheaper than threading a path through the genome, which
    /// has no business knowing where it lives.
    paths: Vec<PathBuf>,
    /// Which languages have edits not yet written. Parallel to `languages`.
    dirty: Vec<bool>,
    /// The gloss box, for the selected word.
    gloss_draft: String,
    /// The add-a-word boxes.
    new_form: String,
    new_gloss: String,
    /// The declare-a-concept boxes.
    new_key: String,
    new_key_gloss: String,
    /// What the last accepted edit did, shown until the next one.
    last_edit: Option<String>,
    /// Warnings the last accepted edit introduced — shown, never fatal.
    introduced: Vec<String>,
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
            paths: Vec::new(),
            dirty: Vec::new(),
            gloss_draft: String::new(),
            new_form: String::new(),
            new_gloss: String::new(),
            new_key: String::new(),
            new_key_gloss: String::new(),
            last_edit: None,
            introduced: Vec::new(),
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
                self.paths.push(path.to_path_buf());
                self.dirty.push(false);
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
                    self.paths.clear();
                    self.dirty.clear();
                    self.selected = 0;
                    self.traced = None;
                }
                // Save is deliberately in the top bar rather than inside the Edit
                // view: it is not part of making a change, it is the separate,
                // explicit act of writing one down. Disabled when there is nothing
                // to write, so the button never lies about having done something.
                let unsaved = self.dirty.get(self.selected).copied().unwrap_or(false);
                if ui
                    .add_enabled(unsaved, egui::Button::new("Save"))
                    .on_hover_text("write this language back to its file — undo is the file")
                    .clicked()
                {
                    self.save_selected();
                }
                if unsaved {
                    ui.label(egui::RichText::new("● unsaved").small().weak());
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
                View::Edit => self.edit_view(ui),
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

    /// M16. Four edits, each one `Edit` value through `apply_edit` — the same call
    /// the CLI makes, which is what keeps the window and the command line from
    /// disagreeing about what an edit means.
    fn edit_view(&mut self, ui: &mut egui::Ui) {
        let Some(genome) = self.current() else { return };
        ui.heading(format!("Edit — {}", genome.name));
        ui.label(
            egui::RichText::new(
                "every change here is the same library call `stemma set-gloss` makes; \
                 nothing is written until you press Save",
            )
            .small()
            .weak(),
        );
        ui.separator();

        // --- the accepted-edit banner, and any warnings it introduced ---
        if let Some(done) = self.last_edit.clone() {
            ui.label(egui::RichText::new(format!("✓ {done}")).strong());
            for warning in self.introduced.clone() {
                ui.label(egui::RichText::new(format!("  {warning}")).small().weak());
            }
            ui.separator();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.gloss_editor(ui);
            ui.separator();
            self.add_word_editor(ui);
            ui.separator();
            self.declare_concept_editor(ui);
        });
    }

    fn gloss_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Gloss");
        let Some(id) = self.traced.clone() else {
            ui.label("Pick a word from the list on the left to relabel it.");
            return;
        };
        let current = self
            .current()
            .and_then(|g| g.lexicon.iter().find(|e| e.id.as_str() == id))
            .and_then(|e| e.display_gloss())
            .unwrap_or("?")
            .to_owned();
        ui.label(format!("{id} — currently \"{current}\""));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.gloss_draft).desired_width(280.0));
            if ui.button("Set gloss").clicked() {
                self.run(stem_genome::Edit::SetGloss {
                    word: stem_core::WordId::new(id.clone()),
                    gloss: self.gloss_draft.clone(),
                });
            }
        });
        ui.label(
            egui::RichText::new("empty clears the override, restoring the concept's own gloss")
                .small()
                .weak(),
        );
    }

    fn add_word_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Add a word");
        ui.horizontal(|ui| {
            ui.label("Form:");
            ui.add(egui::TextEdit::singleline(&mut self.new_form).desired_width(140.0));
            ui.label("Gloss:");
            ui.add(egui::TextEdit::singleline(&mut self.new_gloss).desired_width(200.0));
            if ui.button("Add").clicked() {
                self.run(stem_genome::Edit::AddWord {
                    form: self.new_form.clone(),
                    gloss: self.new_gloss.clone(),
                    concept: None,
                    part_of_speech: stem_lexicon::PartOfSpeech::Noun,
                });
            }
        });
        ui.label(
            egui::RichText::new(
                "the form is read against this language's own inventory — a sound it \
                 has not got is refused, and says which",
            )
            .small()
            .weak(),
        );

        if let Some(id) = self.traced.clone() {
            ui.horizontal(|ui| {
                ui.label(format!("Selected: {id}"));
                if ui.button("Remove this word").clicked() {
                    self.run(stem_genome::Edit::RemoveWord {
                        word: stem_core::WordId::new(id),
                    });
                    self.traced = None;
                }
            });
        }
    }

    fn declare_concept_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Declare a concept");
        ui.horizontal(|ui| {
            ui.label("Key:");
            ui.add(egui::TextEdit::singleline(&mut self.new_key).desired_width(140.0));
            ui.label("Gloss:");
            ui.add(egui::TextEdit::singleline(&mut self.new_key_gloss).desired_width(200.0));
            if ui.button("Declare").clicked() {
                self.run(stem_genome::Edit::DeclareConcept {
                    key: stem_lexicon::ConceptKey::new(self.new_key.clone()),
                    gloss: self.new_key_gloss.clone(),
                    part_of_speech: stem_lexicon::PartOfSpeech::Noun,
                    note: String::new(),
                });
            }
        });
        ui.label(
            egui::RichText::new(
                "a meaning the built-in list does not hold; `new-lexicon` will coin a \
                 word for it",
            )
            .small()
            .weak(),
        );
    }

    /// Applies one edit, or shows why it was refused.
    ///
    /// **The whole of the UI's editing logic is this function**, and it has none: it
    /// hands the value to the library, keeps the result on success, and puts the
    /// error in the banner on failure. A refused edit changes nothing — not the
    /// genome, not the dirty flag — because `apply_edit` returns a new genome rather
    /// than mutating the one it was given.
    fn run(&mut self, edit: stem_genome::Edit) {
        let Some(genome) = self.languages.get(self.selected) else {
            return;
        };
        match stem_genome::apply_edit(genome, &edit) {
            Ok(outcome) => {
                self.languages[self.selected] = outcome.genome;
                self.dirty[self.selected] = true;
                self.last_edit = Some(edit.summary());
                self.introduced = outcome.introduced.iter().map(|i| i.to_string()).collect();
                self.error = None;
            }
            // Refused. The language is untouched and nothing is marked dirty, so the
            // window cannot be holding a change the library said no to.
            Err(e) => {
                self.error = Some(e.to_string());
                self.last_edit = None;
                self.introduced.clear();
            }
        }
    }

    /// Writes the selected language back to the file it came from.
    ///
    /// The one place in this crate that touches the disk, behind an explicit press.
    fn save_selected(&mut self) {
        let (Some(genome), Some(path)) = (
            self.languages.get(self.selected),
            self.paths.get(self.selected),
        ) else {
            return;
        };
        match stem_io::save(path, genome) {
            Ok(()) => {
                self.dirty[self.selected] = false;
                self.last_edit = Some(format!("saved to {}", path.display()));
                self.error = None;
            }
            Err(e) => self.error = Some(format!("could not save `{}`: {e}", path.display())),
        }
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
    /// This crate may *load*, *assemble*, *render* and — since M16 — *save*. It may
    /// not apply a rule, drift a meaning, compose a word or mint an id. The moment it
    /// does, the CLI and this window can disagree about what a word's history is —
    /// and since M4 the whole plan has been that they cannot, because there is
    /// exactly one renderer behind both.
    ///
    /// **`stem_io::save` left the ban list at M16 and nothing else did.** That is the
    /// precise shape of the fence §20.5 asked for: editing arrives as one library
    /// call plus one write, not as a second engine. `apply_edit` is deliberately
    /// *not* banned — calling it is the point — but every `Edit` it takes is a value
    /// this crate builds and the library interprets.
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
                    "::derive",
                    "build_shaped_lexicon",
                    "build_proto_lexicon",
                ] {
                    assert!(
                        !code.contains(banned),
                        "{name}:{} calls `{banned}` — the UI is a second front end onto \
                         the libraries, not a second engine. Editing goes through \
                         `stem_genome::apply_edit`, never through an engine entrypoint \
                         (§20.5, M16)",
                        n + 1
                    );
                }
            }
        }
    }
}
