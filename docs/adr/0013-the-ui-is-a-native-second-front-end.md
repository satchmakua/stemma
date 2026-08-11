# 13. The explorer is a native egui window, and a second front end rather than a second engine

- **Status:** Accepted
- **Date:** 2026-08-10

## Context

M11 adds `DESIGN.md` §10's visual explorer — the first UI, held back until the
engine worked. §20.5 names the risk plainly ("the visual interface could consume all
development time") and prescribes the mitigations this milestone follows: *build CLI
first; use Markdown export as early proof; add UI only after engine MVP works; keep
views read-only before adding full editing.* The first three are done; the fourth is
this ADR's scope fence.

Two decisions were open: what to build it with, and how it relates to the engine.

The user's constraint on the first: **something that runs outside a browser, from the
desktop.**

## Decision

### `eframe` / `egui` 0.36, pinned in the workspace table like every other dependency

| option | why not |
|---|---|
| **Tauri** and other WebView shells | Rejected by the constraint. A system WebView *is* a browser engine — it reintroduces HTML/CSS/JS and a Node toolchain, and ships a UI whose rendering depends on which Edge/WebKit the machine happens to have. |
| **Slint** | Good, but it would add a **second DSL** to a project that has just carefully added one (M10). Two notations to learn, and a markup language whose semantics are not the engine's. |
| **Iced** | Sound choice; retained-mode with an Elm-ish architecture. More ceremony than a read-only viewer needs — messages and update loops for a window that only reads. |
| **GTK / Qt bindings** | System libraries to install, which contradicts `CLAUDE.md`'s "don't reach for a heavyweight dependency for a small need" and this project's "no database, files are the format" posture. |

**`egui` wins on the constraint and the shape of the task.** It draws through
wgpu/glow directly, so the result is one self-contained executable with no WebView,
no JS runtime and no system GUI libraries. Immediate mode is a genuine fit rather
than a compromise: a read-only inspector over borrowed state needs no data binding,
no observers, and no second copy of the model that could fall out of sync with the
genome it is displaying.

The cost, recorded honestly: a ~16 MB binary and a first build of about a minute, both
from the graphics stack. Acceptable for a desktop tool; it would not be for a library.

### The UI holds no logic, and a source-scan test enforces it

Every panel presents a string a **library crate** produced:
`render_word_history`, `render_cognate_table`, `render_family`, `render_profile`.
This crate computes no form, resolves no meaning, applies no rule and mints no id.

That has been the plan since M4, when `render_family` was placed in `stem_genome`
with the note *"the M11 UI must render the identical text through the identical
function"*, and it is why `stem_cli` was kept logic-free for eleven milestones. The
payoff arrives here: **the CLI and the window cannot disagree about a word's
history**, because there is one renderer behind both and a bug in it is visible in
both. A UI that re-derived any of this for prettier display would be exactly the
second-engine defect this project has refused at every layer — a second opinion about
what a word means.

`the_ui_computes_nothing_it_could_instead_ask_a_library_for` bans `apply_rules`,
`apply_drift`, `scoped_cognate_set`, `parse_rule_set`, `compose`, `inflect` and
`stem_io::save` from this crate's sources. The last is the read-only fence: the guard
that makes §20.5's "read-only before editing" a checked property rather than an
intention.

### Text is rendered monospace, deliberately

Every one of those renderers aligns its columns by **character count** — a rule the
codebase enforces through one shared `pad` helper. A proportional font would undo
that work, so the explorer sets a monospace default and shows library output in
scrollable, selectable monospace blocks. The window is a *viewer for the CLI's own
output*, not a reinterpretation of it.

## Consequences

- **`stemma-ui` opens a project, inspects a word trace, and views daughters side by
  side** — M11's three acceptance clauses. Files open by native dialog, by drag-and-
  drop onto the window, or as argv (`stemma-ui proto.ron coastal.ron highland.ron`),
  so it takes the same arguments as the rest of the toolchain.
- **Clicking a word in the list opens its full history**, which is §10.2's "killer
  feature" implemented as literally as the phrase suggests.
- **The crate graph gains one leaf.** `stem_ui` sits beside `stem_cli`, depending on
  the same libraries; nothing depends on it, and removing it would cost the project
  nothing but the window.
- **Deferred, and named so scope cannot creep:** every §10.1 view beyond the four
  built (no phonotactics builder, no timeline, no script view, no translation lab, no
  alien modality designer); all editing, and therefore all saving; project files as a
  concept distinct from a set of open languages; syntax highlighting for `.sc`;
  theming; persistence of window state. §20.5's warning is that this list is where a
  project goes to die, so it stays a list until the engine asks for more.
- **A note for the next session:** the window could not be screenshotted from inside
  a session, because the computer-use allowlist resolves installed applications and
  this is a freshly built dev binary. Verification was: it compiles, launches, holds
  a real window, and its content is library output already pinned by tests elsewhere.
