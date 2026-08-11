//! `stemma-ui` — the desktop explorer (ROADMAP M11, `DESIGN.md` §10).
//!
//! # Why a native window rather than a browser
//!
//! `eframe`/`egui` draws through wgpu/glow directly. There is no system WebView, no
//! JS runtime, and no Node in the toolchain, so the result is a **single
//! self-contained executable** a user runs from their desktop — which is what was
//! asked for, and which also keeps the project's "no heavyweight dependency" rule
//! (`CLAUDE.md`) intact. A WebView-based shell (Tauri and friends) would embed a
//! browser engine and reintroduce exactly the stack this avoids.
//!
//! Immediate mode is the right fit besides: M11 is **read-only** by §20.5's explicit
//! mitigation for UI complexity, and an immediate-mode viewer over borrowed state
//! needs no data binding, no observers, and no second copy of the model to keep in
//! sync with the genome.
//!
//! # This binary is a second front end, not a second engine
//!
//! It holds no logic. Every panel shows a string a library crate produced, through
//! the same functions `stemma` the CLI calls — see [`app`]'s module docs. Open it
//! next to a terminal and the two agree by construction.

mod app;

fn main() -> eframe::Result<()> {
    // Any paths on argv open immediately, so the UI takes the same arguments the
    // rest of the toolchain does: `stemma-ui proto.ron coastal.ron highland.ron`.
    let paths: Vec<std::path::PathBuf> = std::env::args_os().skip(1).map(Into::into).collect();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("Stemma — language explorer"),
        ..Default::default()
    };

    eframe::run_native(
        "stemma-ui",
        options,
        Box::new(move |cc| {
            // The derivations, cognate tables and profiles are all aligned by
            // character count, so the default proportional font would undo that
            // work. A slightly larger monospace is the readable default here.
            cc.egui_ctx.all_styles_mut(|style| {
                style
                    .text_styles
                    .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
            });
            Ok(Box::new(app::App::with_paths(paths)))
        }),
    )
}
