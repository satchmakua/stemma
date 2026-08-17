# PROGRESS — Stemma

A build log of what shipped and the notable decisions behind it. **Keep it honest**
— this is the working memory between build sessions. The forward-looking plan and
acceptance tests live in [ROADMAP.md](ROADMAP.md); this is the backward-looking
"what got done and why" companion.

**Current phase:** **Phase 8 has begun** — M23 gives a language's speakers a body, and
Stemma an honest account of which of its own machinery applies to that body. Next
milestone: **M24 — non-vocal signal systems**, the last one on the roadmap. Phases 0–7
(M0–M22) are complete.

## State of the tree

| Crate | Holds | Status |
|---|---|---|
| `stem_core` | typed IDs (`MorphemeId` M8, `SemanticNodeId` M9), `StemmaError`, `Validate` / `ValidationReport`, `rng`, `suggest` | working |
| `stem_phonology` | features, `Phoneme`, inventory, phonotactics, root generation | working |
| `stem_lexicon` | `WordEntry`, `Lexicon`, the **673**-concept list, cognate-set minting, morphemes / `compose` / `inflect`, senses / `apply_drift` / sense history, derivation patterns / `derive` / `BaseRef`, **culture profile / `build_shaped_lexicon`** | working |
| `stem_soundchange` | rules, matching, ordered application, resolution, traces | working — **untouched by M8 and M9** |
| `stem_syntax` | §7.4's parameters, derived headedness, typological harmony, **propositions / `generate` / constructions** | working (M17, M18) |
| `stem_script` | glyphs, `ScriptKind`, sound/meaning→sign mappings, `write`, lossiness, **glyph biographies / `script_drift`** | working (M20, M21) |
| `stem_assist` | briefings out, proposals in, and one gate between them — no network, no key, no model in the process | working (M22) |
| `stem_embodiment` | §18.1's profile, §18.2's channel constraints, and which subsystems apply to a given body | working (M23) |
| `stem_genome` | `LanguageGenome`, `fork`, `LineageGraph`, family validation, renderers, `apply_edit`, `say`, **`apply_shifts` / syntactic history** | working |
| `stem_io` | RON/JSON load & save | working — **untouched since M0** |
| `stem_export` | Markdown dictionaries, CLDF CSV, cognate table, family demo | working |
| `stem_cli` | the `stemma` binary | …plus `profile`, `inflect`, `paradigm`, `drift`, `drifts`; rule files may be `.sc` |
| `stem_ui` | the `stemma-ui` desktop explorer — native egui; **edits through `apply_edit`** | working (M11, M16) |

---

## M23 — Embodiment profile · built 2026-08-16 · ✓ verified

Phase 8 opens, and it is the one that touches every layer: the data model has assumed a
vocal tract since M0. **833 tests pass** (up from 799); clippy clean; fmt clean.

```
$ stemma validate fixtures/luminous_kethi.ron

Kethi (luminous_kethi) — 0 phonemes (0C/0V), proto, seed 108

· embodiment.vocal_checks_set_aside: these speakers have no vocal tract, so `empty`,
  `no_templates`, `no_syllable_counts` do not apply and were set aside
· embodiment.simultaneous_channel: `mantle` can carry several parameters at once…
· embodiment.persistent_channel: `ink` persists after it is made…

✓ valid — 0 warning(s), nothing blocking
```

```
$ stemma embodiment fixtures/luminous_kethi.ron

  What Stemma can do with this body
    Phonology     does not apply  a phoneme inventory is a set of things a vocal tract
                                  can do, and these speakers have no vocal tract…
    Phonotactics  does not apply  a `CVC` template describes a syllable…
    Prosody       does not apply  stress is a property of a syllable
    Sound change  not built yet   signal change over this channel's own contrastive
                                  dimensions is M24's work
    Morphology    applies         composing meaningful parts is about structure, not
                                  about a mouth
    Semantics     applies         meaning and its history are independent of the channel
    Syntax        applies         …though §18.3 expects a non-vocal channel to solve it
                                  differently
    Script        partly          a sign may map from a meaning; a sign that maps from a
                                  phoneme has nothing to map from here
```

### What M0 already knew

`PhonemeInventory::validate` has errored on a vowelless inventory since the first
commit, and that error's message has always ended *"(a non-vocal language will need the
alien modality model of §7.7)"*. M23 is where the parenthesis comes due.

The Error that actually blocked the Kethi turned out to be `empty` rather than
`no_nucleus` — an inventory with no phonemes returns early — and that widened the rule.
`VOCAL_TRACT_CHECKS` is now *"a check that is only meaningful if the speaker produces
sound at all"*, which covers `empty` and the phonotactics pair as well as the C/V ones.

**What stays** is everything about a well-formed *record*: `duplicate_id`, `empty_ipa`,
`bad_weight`. `a_broken_record_cannot_hide_behind_a_claim_to_be_alien` is the test that
stops `embodiment:` becoming a way to silence anything inconvenient, and it is the
reason this is one short named list with a printed reason rather than a suppression
mechanism.

### Silence is not a claim to be alien

`EmbodimentProfile::has_vocal_tract` returns **`true` for an empty profile**, and that
one line is what keeps the whole project working. Every language written before M23 says
nothing about its speakers, and every one of them has speakers with mouths. Had silence
read as alien, M23 would have set aside the inventory checks for the entire corpus —
including the `no_nucleus` Error that catches a genuinely broken human language.
`silence_is_not_a_claim_to_be_alien` sweeps three fixtures, `invalid_no_vowels.ron`
among them, and asserts it is *still invalid for the reason it always was*.

### "rather than silently producing vowels"

The applicability table says what *should* apply. The half that needs the genome is what
*did*: `check_against_language` compares the two and reports the overlap — a non-vocal
species carrying five vowels, syllable templates, a stress policy, a lexicon coined from
machinery that does not apply to it. M19's discipline in its fourth outing: the profile
proposes, the engine observes.

Reported, never refused. An author converting a language gives it a body before
rewriting its inventory, and refusing to load the intermediate state would make the
conversion impossible to do a step at a time.

### One `Vec<Channel>`, not five

§18.1 sketches four parallel `Vec`s — visual, chemical, tactile, electric/magnetic. This
crate carries one `Vec<Channel>` whose members name their own `ChannelKind`, and the
deviation is recorded in the module docs:

- §18.2 gives every channel the *same* ten constraints, so the four structs would have
  been identical but for the pile they sat in;
- the medium is a property of the channel, not a filing decision;
- four parallel lists is the shape that guarantees a renderer eventually iterates three
  of them.

Four of §18.2's constraints drive a reported §18.3 consequence (persistence,
simultaneity, directionality, privacy); the other six are bands that are **printed and
nothing else**, and the sketch says so rather than letting an author think a value they
filled in drives something it does not.

`VocalTractProfile` is deliberately almost empty. A language with a vocal tract already
describes it in exhaustive detail — that is what `PhonemeInventory` *is* — and a second
description would be two sources of truth about one anatomy. What matters is that the
field is an `Option`, and `None` is the whole alien case.

### The bug the nesting created, and the fix

§18.1 makes `environment: EnvironmentProfile` a field **of** the embodiment profile, and
M15 put it on the genome two milestones before there was an embodiment profile to put it
in. Nesting it gave the same type two homes — and the Kethi fixture, which follows §18.1,
had its ecology **silently ignored**: `stemma profile` read "unshaped (no culture
profile)" for a species with two declared absences.

`LanguageGenome::ecology()` is now the one place either field is read, every caller
routes through it, the nested one wins, and declaring both is a Warning
(`two_ecologies_declared`) so it is never resolved silently. That failure — following the
design document and watching your work do nothing — is the class this project spends
most of its comments guarding against, and it was worth catching before the fixture
shipped.

### §17's last row

`AlienEmbodimentDependence` left `NOT_MODELLED`, which is now **empty**. Every dimension
§17 names has a band, for the first time since M7 deferred four of them.

The block is *kept rather than deleted*, and prints `(none — every dimension §17 names
is scored above)`. It is where the profile admits what it cannot measure, and a heading
that vanishes when the list empties is one nobody notices going missing when it refills.
`NotModelled` survives as an uninhabited enum for the same reason.

The band separates `NonVocal` from `Mismatched`: a body Stemma has no machinery for, and
a body that has been given machinery it has no use for anyway. The second is a finding
about the *file* rather than about the species, and reads differently.

### Invariants worth carrying to M24

- **An empty profile is a vocal speaker.** Everything else follows from it.
- **`VOCAL_TRACT_CHECKS` is a short named list with a printed reason**, and the only
  place anything is filtered out of a sub-report. Record-level checks are never on it.
- **§18.3's consequences are Notes, argued from physics, never counted.**
  `no_consequence_message_claims_a_statistic` strips `§N.N` citations and then scans for
  digits — the citations `CLAUDE.md` asks for are not statistics, and M17's bare sweep
  would have banned them.
- **`ecology()` is the one reader.** Do not add a second.
- **`stem_embodiment` names no signal type.** It models the body; generalising a phoneme
  to a channel signal is M24, and doing it here would have been building the interesting
  half before the boring half was honest.

### What is deliberately unbuilt

**Signals.** The Kethi have two channels and nothing to say on either — `phonemes: []`,
no lexicon, no rules. Filling the inventory with fifteen sounds so `new-lexicon` would
run is precisely the failure the acceptance is written against. M24 generalises a phoneme
to a channel signal and gives them something to say.

**Inferring grammar from a body.** §18.3's consequences are *reported*. Stemma does not
go looking for simultaneous morphology in a language with a simultaneous channel, and
does not mark its absence — the Kethi are allowed to be strange in ways their body does
not predict, exactly as M17's languages are allowed to be typologically disharmonic.

---

## M22 — Constrained LLM assistant · built 2026-08-16 · ✓ verified

Phase 7, and the phase with the sharpest constraint on it. **799 tests pass** (up from
785); clippy clean; fmt clean.

```
$ stemma accept out/proto.ron --proposal fixtures/proposal_raising.ron \
      --id raised --name "Raised Asterian" --years 450 --out out/raised.ron

Proposal `raising_and_voicing` — for proto_asterian
  `rules_raising` — 2 rule(s)  ·  a sound-change rule set
  written by claude-opus-5 (from `stemma brief fixtures/proto_asterian.ron --for rules`)

  ── the proposer's own words, not the engine's ──
  │ Two changes: one that destroys a contrast and one that creates one.
  │ …
  ── nothing above was parsed, matched, or stored ──

  accepted — `rules_raising` — 2 rule(s) applies cleanly and would produce a traced,
  reproducible language

  note: [soundchange.phoneme_innovated] the language gained /b/ (`ph_b`) — phonemic split…
```

And the other half:

```
$ stemma accept out/proto.ron --proposal fixtures/proposal_incoherent.ron …

  refused — the rule set `rules_assimilation` cannot be applied; nothing was applied

  error: [rules.change_references_unmatched_position] the change copies from After(0),
  but the environment declares no slot there (r_a01)
```

No file was written.

### The architecture is decided by the constraint

*No LLM output may mutate a language without passing through validation* (§3.2),
enforced since M0 when there was nothing to enforce it against. The strongest possible
enforcement of "the model is not the source of truth" is that **the model is not in the
process** — so `stem_assist` has no HTTP client, no API key, no `async`, and nothing
that can fail at a network boundary.

Stemma writes a **briefing** and reads back a **proposal**: a file, in the same formats
a human writes, which the engine applies by the ordinary code path or refuses. The
model runs wherever the user likes. That is not a limitation dressed up as a principle
— it is what makes the guarantee checkable, and it keeps `cargo test --workspace`
offline and deterministic, which an in-process model would have ended.

It also satisfies the roadmap's third clause exactly: *with the assistant disabled,
every other command behaves identically, byte for byte.* Operationally, **the assistant
added no genome field**. A `proposed_by` stamp was the tempting alternative and is
refused on purpose — an accepted rule set is the same artefact whichever hand wrote it,
and a field recording otherwise is the engine learning a distinction §3.2 forbids it to
make. `the_assistant_added_no_field_to_a_language` scans saved files for it.

### The load-bearing test is not the refusal

The constraint does not die by someone writing a bypass. It dies by someone adding a
*second* path: a fast route for machine-written artefacts, a validator tuned for what
models tend to get wrong, a flag that skips a check because the briefing already
covered it. Each is reasonable alone; each ends with two code paths, one less examined
than the other.

So the test that carries this milestone is
`an_accepted_proposal_is_byte_identical_to_the_same_rules_typed_by_hand`: the same
`RuleSet` through `stem_assist::accept` and through `LanguageGenome::evolve` must
serialise to identical bytes. If a difference ever appears there, a second path exists
whatever the docs say.

`the_assistant_never_does_the_engines_job` is the structural half — a source scan, the
shape of `stem_soundchange`'s three guards. `stem_assist` may not name `WordEntry`,
`Derivation`, `scoped_cognate_set`, `StemmaRng` or `apply_rules`. It is an envelope and
a gate; everything else goes through `evolve` / `drift` / `apply_edit`.

### The envelope, and why prose needs somewhere to go

A `Proposal` is **artefact + provenance + rationale**, kept apart:

- `artefact` is the only thing the engine sees — a `RuleSet`, a `DriftSet`, a concept
  list, with no field marking it machine-made.
- `rationale` is the model's own words. Never parsed, never matched, never stored.
- `provenance` says who wrote it, for the reader, never for the code.

Without the envelope a model's reasoning ends up in a `note:` field *inside* the rule
set — stored in the language file, indistinguishable from the author's own. That is
what "prose it writes is labelled as prose" costs structurally, and
`the_proposers_prose_is_labelled_as_prose_and_never_stored` hunts four distinctive
strings from the fixture's rationale through the saved daughter.

The rationale prints **above** the verdict, fenced and attributed, because that is the
order it happened in: a claim, then a check of the claim. Printed underneath an
"accepted" it would read as though the engine had endorsed the argument.

### `review` is `accept` with the result thrown away

Not a re-implementation of the engine's checks — the **actual application against a
clone**, discarded. M16's validate-a-clone rule, and here it makes disagreement
impossible by construction: a gate that could say "accepted" where accepting then
refused would be theatre. `a_review_never_disagrees_with_what_accepting_would_do`
holds both fixtures to it.

### The fixtures are real model output, and one of them was wrong

Both were written by Claude (Opus 5) from `stemma brief`, in this session, and say so
in their headers. A fixture claiming a provenance it does not have would be the exact
dishonesty this milestone exists to prevent.

**The first draft of the accepted one was wrong**, and that is recorded in the file. It
proposed velar palatalization — /k/ fronting to /t/ before a front vowel — and argued
it well. The engine accepted it and then reported `unnameable_output` on every site:
setting `[-dorsal, +coronal]` on /k/ leaves the dorsal-dependent `+back` and `-round`
still specified, and no phoneme and no reference row carries that bundle. The prose was
right about the sound change and the formalism did not do it. Nothing but running it
through the engine would have shown that.

The replacement proposes a merger and a split, and makes a **checkable** claim: that
raising the mid vowels would create homophones. `stemma validate` reports 62 shared
written forms before and 79 after. The claim was testable and it held.

**The refused fixture is the interesting failure.** Its prose is correct linguistics —
nasal place assimilation, §7.2's own example, accurately described — and its rule
copies place from `after[0]` while the environment declares no `after` slot. One
missing line in a file where every other line is right. Reading the rationale finds
nothing wrong; only running it does. That is the entire argument for the gate.

And the refusal is **whole**: the set's second rule is fine and does not run either.
`a_refusal_takes_the_good_rules_down_with_the_bad_one` proves the second rule applies
cleanly on its own, so its failure in the set is the refusal being whole rather than
the rule being bad.

### One crash fixed on the way

Adding three subcommands pushed the clap-derived `Command` tree past Windows' 1 MiB
default main-thread stack **in debug builds** — every invocation overflowed, `--help`
included, before a line of Stemma's own code ran. Release builds inlined it away and
looked fine.

That mattered beyond comfort: the acceptance tests shell out to `CARGO_BIN_EXE_stemma`,
which is the *debug* binary, so the whole suite would have gone down. `main` now runs
`run()` on a thread with a 16 MiB stack, with the reasoning recorded on the constant —
the command set growing is the normal course of this project, so the fix belongs there
rather than in a diet on the help text.

### Invariants worth carrying to M23

- **One apply path, and it is the authored one.** Every arm of `accept` hands the
  artefact to the function an authored file would have reached.
- **No genome field, ever, that records how an artefact was written.**
- **`review` is a dry run of `accept`, not a second opinion.**
- **Prose lives in the envelope**, printed fenced and attributed, and never in a
  language file.
- **A proposal names its target**, and one aimed at another language is refused before
  anything is tried.

### What is deliberately unbuilt

**In-process model invocation.** No HTTP client, no key handling, no `--model` flag.
Adding one would make the test suite network-dependent and nondeterministic, and would
put an API client in a tool whose project format is files (§19.2). The out-of-process
boundary is the enforcement mechanism, not an omission — and a user who wants
automation can pipe `stemma brief` into any assistant and `stemma review` the answer.

**§6.5's "translate small phrases".** `stemma say` (M18) already puts a proposition
through the formal grammar; a model translating *free* text would be producing surface
forms the engine never made, which is the one thing that must not happen.

---

## M21 — Script evolution · built 2026-08-16 · ✓ verified

Phase 6 closes with §7.6's real claim made real: **a glyph has a history of its own,
and it comes apart from the language's.** **785 tests pass** (up from 761); clippy
clean; fmt clean.

§7.6's worked chain, walked:

```
$ stemma glyph-trace out/written.ron ge_star

★  ge_star — the star
  The Emmen signs (logography)

  0  pictogram      ✶    = STAR
                  Six scratches from a common centre. A drawing of the thing.
  1  logogram       ✶    = STAR
                  The same shape, but now a WORD rather than a picture.
  2  determinative  ✶    (neither sound nor meaning)
                  Silent. Written before a god's name, and never read aloud.
  3  rebus sign     ✶    /s/
                  The hinge: borrowed for the SOUND its old word began with.
  4  phonogram      ★    /s/
                  Three strokes, written fast with a reed.
  →  now            ★    = STAR
```

And then the half that makes it *evolution* rather than a biography field:

```
$ stemma apply-rules out/written.ron --rules fixtures/rules_glide_loss.sc \
      --id asterian_late --name "Late Written Asterian" --years 450 --out out/written_late.ron
$ stemma glyph-trace out/written_late.ron g_w --script kirran

w  g_w — the hook
  0  pictogram   ʖ    = WATER
  1  rebus sign  ʖ    /w/
  →  now         w    /w/

  This sign has outlived its sound: no word of Late Written Asterian contains what it
  writes any more, and it is still on the page. It is recorded writing /w/ in the past.
```

### The clause the milestone turns on

§7.6 says *a glyph should have ancestry just like a word* — and that the two are
**independent**. The second half is the whole milestone. A biography hung on a sign is
a `Vec` with prose in it; any file format can hold one, and holding one proves nothing.
What makes this script *evolution* is that the sign's history and the language's are
measured separately and observed to come apart.

So the fixture does it the only honest way. `fixtures/rules_glide_loss.sc` is two
ordinary sound changes that **have never heard of the script**: glide loss deletes
/w/ and /j/, intervocalic voicing mints /b/ /d/ /ɡ/. They name feature bundles. They
would run identically on a language nobody had ever written down. What they do to the
Kirran alphabet is *found afterwards*:

- four signs (`g_w`, `g_y`, `gt_w`, `gt_y`) are left writing sounds no word contains;
- three new sounds have no letters, and 125 words can no longer be spelled in full.

Two tests attack the alternative. `nothing_in_the_change_file_mentions_the_script`
scans the rule directives for `glyph`, `script`, `kirran`… — and the structural version
that actually lasts is a new source-scan guard,
`the_engine_never_references_script`, the **third** of `stem_soundchange`'s guards
after morphology (M8) and semantics (M9). The engine cannot name a glyph type, so a
rule cannot be written to target one, so the drift cannot be staged.

### The trap: the inventory is the wrong place to ask

`apply_rules` only ever **grows** an inventory — a phoneme stays in it after the last
word containing it changed, so earlier trace steps keep resolving. Asking the inventory
"does this language still have /w/?" therefore answers *yes* forever, and the fossil
finding would never fire once.

Every count in `script_drift` comes from the **lexicon**.
`the_finding_comes_from_the_lexicon_and_not_from_the_inventory` pins it by asserting
both halves at once: /w/ is still sitting in the daughter's inventory, no word contains
it, and the sign is reported as a fossil anyway. That is the test that would catch a
future "simplification" to the obvious-looking check.

### A stage is a `RuleApplicationTrace`, and the present is not one

`Glyph::history` is `Derivation`'s shape exactly: `history[0]` is the pictogram the way
`Derivation::input` is the proto-form, the entries are the steps, and **the present is
not in there** — the current shape is `Glyph::form` and the current job is whatever the
script's `mappings` say. Storing the present twice is the desynchronisation M2 banned
when it refused to keep a rendered string beside `phonemic_form`, and the fixture
demonstrates why it matters: `ge_star`'s last *recorded* stage is a phonogram for /s/,
while its job *today* is logographic. The two differ, and only one of them is stored.

An empty `form` on a stage means the shape did not change — most stages change the job
only — and the renderer prints nothing rather than filling in the current shape, which
would claim a redraw that never happened.

**No stage carries a year.** M19's rule: a trigger is verified, never asserted. A
glyph's biography is authored prose the engine cannot check, so a date would dress an
assertion up as a measurement. The order is the claim, and order is all §7.6 states.

### §17's last-but-one row

`ScriptHistoryCoherence` left `NOT_MODELLED` — M7's deferred list is down to
**one** row (alien embodiment, §18). `ScriptHistory` is a band over the same
`DEEP_ORTHOGRAPHY` the `deep_orthography` Note reads, so band and Note cannot disagree
(`docs/adr/0009`, fourth instance). The reference daughter reads `historical` at five
mismatches and sits deliberately below the bar of eight — M8's rule that the tool's own
showcase must not trip the extreme threshold.

The constant is a deliberately loose tripwire and says so: there is no attested
"orthographic depth" scale, and inventing one would be the fabrication §17 forbids.

### The lie the logography nearly told

The first working version reported the Emmen signs as *"the spelling still matches the
pronunciation"* — and it passed its tests, because `script_drift` correctly returned
zero for it. But a logography encodes no sound; crediting it with keeping up is
crediting it with winning a race it is not running, and `Phonemic` in the profile would
have claimed every sound had a sign in a script that has no sound signs at all.

Three places now say the true thing instead, and one band value exists for it:

```
    writes no sounds, so no pronunciation can leave it behind
    This sign writes no sound, so no sound change can strand it — which is why signs
    like it outlast the pronunciations around them.
    Script history   sound-independent  (writes meanings; no pronunciation to drift from)
```

That independence is not a gap in the model. It is the reason logographic scripts
outlive the pronunciations around them, and it deserved a sentence rather than a zero.

### Invariants worth carrying to M22

- **The drift is a finding, not a field.** Nothing in any fixture declares that a
  script fell behind; `script_drift` computes it from the word list. That is the M19
  discipline — author proposes, engine verifies — and M22's LLM assistant needs it
  intact, because a model that could *assert* a finding could assert a false one.
- **`the_engine_never_references_script`** joins the morphology and semantics guards.
  Three source scans now hold `apply_rules` to phonology.
- **Every script issue is a Note or a Warning.** A drifted orthography is the normal
  fate of writing, not a defect: English is deep and is not thereby broken.
- **A glyph id is script-scoped**, so `resolve_glyph` refuses an ambiguous one and
  names the scripts rather than tracing the wrong sign's biography.
- **`history` is additive.** An M20 glyph with no biography serialises with no
  `history` bytes at all.

### What is deliberately unbuilt

**Cross-glyph ancestry** — one sign descending from a sign in *another* script, the way
Latin `A` descends from Phoenician *aleph*. M21 builds the same-identity chain §7.6's
example describes and the roadmap's test asks for ("walks a glyph back to its
pictogram"). Script *borrowing* is a contact fact between two writing systems, and
modelling it without a donor script in the file would be M15's borrowed-looking
words again.

**A logogram whose meaning the language lost.** A real kind of fossil, but it arrives
through semantic drift (M9) rather than the sound change this milestone measures, and
answering it properly needs sense reasoning rather than a mapping lookup. `fossils` is
phonographic signs only, and the doc comment says so.

---

## M20 — Glyphs & writing systems · built 2026-08-15 · ✓ verified

Phase 6 opens: a language can be **written down**, and the tool says what the writing
lost. **761 tests pass** (up from 728); clippy clean; fmt clean.

```
$ stemma new-lexicon fixtures/written_asterian.ron --out out/written.ron
$ stemma write out/written.ron star

sosem
  sosem  "star"  ·  The Kirran alphabet (alphabet)

  s     /s/           [g_s]
  o     /o/           [g_o]
  s     /s/           [g_s]
  e     /e/           [g_e]
  m     /m/           [g_m]

  every sound is written; this spelling can be read back exactly.

$ stemma write out/written.ron star --script tirran

SSM
  sosem  "star"  ·  The Tirran abjad (abjad)

  S     /s/           [gt_s]
  S     /s/           [gt_s]
  M     /m/           [gt_m]

  Not written:
    /o/  — by design
    /e/  — by design

  2 sound(s) are not written, which is what an abjad does — a reader supplies them
  from knowing the language. This spelling does not round-trip, and is not meant to.
```

One phonology, one seed, three scripts. Everything that differs between the spellings
is attributable to the mapping — the M15 (two ecologies) and M18 (two grammars) shape
again.

### The clause the milestone turns on

ROADMAP M20's acceptance ends *"rather than pretending round-trip."* Writing a word in
a script is a lookup table and would be a morning's work. What makes it a milestone is
that a script is **allowed to lose things**, and there are two comfortable lies
available:

1. **Invent the missing signs** so the round trip works. An abjad with vowel letters is
   an alphabet, and the author asked for an abjad.
2. **Drop them quietly**, so `SSM` reads as a complete spelling of *sosem*.

So `Written` carries `unwritten` alongside `glyphs` **every time**, `lossiness()` states
which situation you are in **including when nothing was lost**, and the round-trip claim
is *measured* rather than announced: `the_round_trip_claim_is_true_of_the_alphabet_and_false_of_the_abjad`
replays each sign's `covers` into a segment list and compares it with the word.

A report that only ever spoke up about failure would let silence read as completeness —
which is lie (2) wearing a different hat.

### A glyph is an entity, not a character

§7.6's claim is that *a glyph should have ancestry just like a word*, and that decides
the model. `Glyph` is `{ id, form, name, note }` — `form` is how it is drawn **today**,
`id` is what it **is**. `a_glyph_keeps_its_identity_when_its_form_is_redrawn` redraws
every sign in the abjad and asserts the same glyphs still wrote the word: the page looks
different, the mapping is untouched. That identity is what M21's descent hangs from, and
it would be impossible if the character were the identity.

`GlyphId` joins the typed-id family in `stem_core` with prefix `g`.

### What the `kind` is for, and what it is not for

`ScriptKind` does **not** decide the mapping — the author declares that sign by sign.
The kind decides exactly one thing: what counts as *expected*. An abjad with no vowel
signs is doing its job (a Note naming the design); an alphabet with none has a hole (a
Warning naming the gap). Same fact, two readings, and
`a_hole_in_an_alphabet_reads_as_a_gap_and_not_as_a_design` pins that they never print
the same sentence. `Unwritten::expected` is a field rather than a judgement made at the
point of printing for that reason.

### The logography was nearly scaffolding

The first draft declared `Mapping::Concept` and `ScriptKind::Logography` — and nothing
consumed either. A grep during the inline review found it: `write` never looked at the
`Concept` variant, so `kind: logography` was a label that changed nothing observable,
and a logographic script would have spelled nothing while reporting all ten of its
consonants as *gaps in the mapping*. Exactly the failure M19's notes name ("a
`Consequence` that changed nothing observable would be scaffolding"), and logography is
in M20's own scope line.

So it is built. A logogram wins outright over the letters, covers no sounds, and puts
**every** segment in `unwritten` — because you cannot read a pronunciation off `★`.
`ScriptKind::expects_unwritten(is_vowel)` replaced `expects_unwritten_vowels()`: a
logography answers `true` regardless, since asking per-vowel would report its consonants
as holes in a mapping that was never phonographic.

### Written nothing ≠ written incompletely

Running it turned up a second lie the first version told. A logography has signs only
for the words that earned one; ask for `king`, which has none, and the output was an
empty spelling explained as *"5 sound(s) are not written, which is what a logography
does — a reader supplies them from knowing the language."* False on both halves: nothing
was written, and there is nothing for a reader to supply *from*.

Two fixes. `write_with_inventory` refines the kind's general answer with the per-word
fact (`expected` is true for a logography only when a sign actually fired), and
`lossiness` gets a case ahead of the others:

```
`emmen` has no sign for this word, so none of its 5 sound(s) reached the page at all.
Nothing was written — which is not the same as something written incompletely.
```

The renderer names the empty spelling too, rather than leaving a blank line that reads
as a rendering bug. The spelling line itself stays genuinely empty: a placeholder
character there would be inventing a sign.

### Reported, never enforced

`written_asterian.ron` has an abjad that cannot write five of its fifteen sounds and a
logography that cannot write any of them, and the file is **valid** —
`a_script_that_cannot_write_the_language_is_a_note_and_not_an_error`. Two `lossy_script`
Notes, no Warnings, no Errors. §17 again.

The two silences get different explanations: the abjad `writes no vowels`, the logography
`writes meanings rather than sounds`. Reusing one message for both would be the tool
asserting something false about a script kind in order to save a branch.

### Invariants worth carrying to M21

- **Every sound is accounted for**: written or listed as unwritten, never simply gone.
  `no_sound_is_ever_lost_without_being_named` sweeps all three scripts × 673 words.
- **The abjad writes no vowel anywhere in the lexicon** — checked over the whole
  lexicon, not one convenient word, because inventing a sign is the failure this
  milestone exists to prevent.
- **A logogram is keyed by `ConceptKey`, never `WordId`** — the M14 rule, which applies
  to scripts too: `w_0080` means whatever the concept list holds at position 80, so an
  append would silently repoint every sign in the block.
- **`scripts` is additive.** `proto_asterian.ron` gains no `scripts` bytes on a save
  round trip, pinned by `a_pre_m20_file_loads_and_saves_without_gaining_a_scripts_block`.
- **Naming a script that does not exist errors** rather than falling back to the first.
  A language may have several, and writing in the wrong one quietly is worse than saying
  so.
- **`stem_script` names no engine type.** `write` is pure, total and RNG-free, and the
  crate sits above `stem_lexicon` (for `ConceptKey`) and `stem_phonology`, below
  `stem_genome`. The renderer lives in `stem_genome::script_view`, the `render_grammar`
  precedent.

### What is deliberately unbuilt

**Morphographic mapping.** §7.6 mentions it and it is real — Chinese radicals, Egyptian
determinatives — but a morphographic sign needs a morpheme to point at, and this
project's morphemes are language-scoped citation forms rather than the shared components
a determinative system uses. Shipping a `Mapping::Morpheme` that pointed at a citation
form would be the same scaffolding the logography nearly was.

**Glyph ancestry.** M20 gives a glyph the *identity* that lets it descend from
something. M21 gives it the descent, and with it §17's script-history row — M7's last
deferred plausibility dimension.

---

## M19 — Syntactic change · built 2026-08-12 · ✓ verified

Phase 5 closes with §7.4's closing claim made real: **syntax evolves, and the cause is
on the record.** **728 tests pass**; clippy clean; fmt clean.

```
$ stemma apply-rules out/old.ron --rules fixtures/rules_case_erosion.sc \
      --id middle --name "Middle Asterian" --years 600 --out out/middle.ron
$ stemma shift out/middle.ron --changes fixtures/shift_asterian.ron \
      --id modern --name "Modern Asterian" --years 60 --out out/modern.ron

Syntactic history — Modern Asterian

  0  Word order fixes as the ergative erodes  (sx_0001)
     word order became SOV
     because `m_erg` surfaces on no word of this language; `r_e02` is the recorded
     sound change that erased it
     the cause is on the record: sound change `r_e02`
```

And the language changes with it:

```
                word order   alignment              SEE(KING, STAR)
  Old Asterian  free         ergative-absolutive    mostair ponti sosema
  Modern        SOV          neutral                most sosem pont
```

### The clause the whole design turns on

ROADMAP M19's acceptance ends *"— not asserted by the author."* That decides
everything. It would have been trivial and worthless to let an author write "at year
640, word order becomes SVO"; the claim worth making is that the shift happened
*because* a sound change destroyed the case marking, and a claim like that is only
worth anything if the program can check it.

So the work is split where the knowledge actually is:

- The **author** proposes a consequence — *when the ergative is gone, order fixes to
  SOV*. No engine can derive that. Real languages that lose case go several different
  ways, and which one this people went is a fact about them. Compiling a choice in
  would be M15's ecology→vocabulary inference table, one layer up.
- The **engine** establishes the antecedent. It inflects **every noun in the lexicon**
  with the named affix, runs the language's own recorded sound changes over the
  results, and looks at what is left. If the marker survives anywhere, the change is
  refused and says on which word.
- **Neither writes down the rule.** That is found by replaying each word's derivation
  one step at a time to see which step emptied the affix's span — `render_paradigm`'s
  technique, put to a new use.

That is the same relationship a `RuleSet` has to the sound-change engine and a
`DriftSet` has to `apply_drift`: authored data, applied or refused by code. It is also
— deliberately — the exact shape M22's LLM assistant needs.

### Decisions worth knowing

**A trigger is never a date.** `Trigger::CaseMarkerLost` is the only variant, and a
date-based one would be an assertion wearing a condition's clothes.

**Probing is the measurement, not a proxy for it.** Composing the affix onto real
nouns and running the real history is exactly what a speaker of that stage does when
they inflect a noun. Running the affix's citation form *alone* would have been cheaper
and wrong — sound changes are conditioned by environment, and `-a` survives after a
consonant and vanishes after a vowel under one rule.

**The probe starts from `Derivation::input`, not from the current form.** That field
is documented as "the form entering the first rule this word ever met", so composing
from it and running the whole history reproduces the stage correctly. Composing from
`phonemic_form` — which has already been through those rules — would apply the history
twice and report erosion that never happened. This was caught while writing the
module, not by a test, and it is the kind of error that produces plausible output.

**Lost means lost everywhere.** A marker that erodes in some environments and not
others is *allomorphy*, which M8 already models and which is not a syntactic event.

**Refusal is reported per change.** "Why did this not apply?" is M10's diagnostic
discipline, and it is the question an author has when nothing happened. A run in which
nothing fired also earns a Warning, so a silent no-op is impossible to miss.

### Rule order reaches the grammar

`-ir` ends in a rhotic, so final-vowel loss alone leaves it intact; the rhotic has to
go first, and only then is the stranded `-i` a word-final vowel like any other.
`reordering_the_sound_changes_stops_the_shift_from_firing` swaps the two rules with
M16's `move_rule` and shows the ergative surviving — so the shift is refused.

M3 proved rule order was observable in one word (`tag` vs `tak`). This is the same
fact deciding whether a language keeps its case system.

### What is deliberately not built

M19 names three examples. **Only case erosion is implemented.** Topic markers becoming
articles and serial verbs becoming auxiliaries are not a matter of effort: this
project has no article, no auxiliary and no serial verb in its model, so "becomes an
article" could only be a string. A `Trigger::TopicMarkerLost` with a
`Consequence::BecomesArticle` that changed nothing observable would be scaffolding
pretending to be a feature — what `docs/adr/0008` refused for `LineageEdgeKind` and M4
refused for `WordEntry.ancestor`. They arrive when there is a category to become.

---

## M18 — Constructions & sentence generation · built 2026-08-12 · ✓ verified

**The first sentence.** Every milestone before this one produced words; this one
produces an utterance. **711 tests pass**; clippy clean; fmt clean.

```
$ stemma say out/sov.ron 'SEE(KING, STAR)'
mostair sosema ponti
  SEE(KING, STAR)  ·  Asterian (grammar)

  mostair  agent      king-ERG  [w_0102 · cog_asterian_grammar_0102]
  sosema   patient    star-ABS  [w_0080 · cog_asterian_grammar_0080]
  ponti    predicate  see       [w_0072 · cog_asterian_grammar_0072]

Constructions:
  0  case_marking  marked the agent `ERG`
     because alignment = ergative-absolutive
  1  case_marking  marked the patient `ABS`
     because alignment = ergative-absolutive
  2  clause  ordered the clause SOV
     because word_order = SOV
```

Every word in it is a real lexicon entry with a real id and a real cognate set — so
each one already has a sound-change history and an etymology, and `stemma trace` takes
you straight from a sentence to any word's past.

### The acceptance is about *why* the two differ, not that they do

Two languages producing two sentences is trivially satisfiable by two languages with
different words. So `grammar_asterian.ron` and `grammar_svo_asterian.ron` share a
phonology, a concept list **and a seed** — they coin identical words. Every difference
in the output is therefore grammatical:

```
SEE(KING:BIG, STAR/PRIEST)

  head-final:    mostair sa taot sosema ponti
  head-initial:  sa mostau ponti sosemta taot
```

`the_same_words_come_out_in_a_different_order` pins it by comparing the *multiset of
word ids* used: identical in both, in different order.

**Alignment is the sharpest of the four differences, because it is not a reordering
at all.** An ergative language marks the agent of a transitive clause and leaves the
lone argument of an intransitive one absolutive; a nominative language marks both the
same. So:

```
                 transitive     intransitive
  ergative:      mostair        mostaa          ← two different endings
  nominative:    mostau         mostau          ← one
```

That is the definition of the two terms, produced by the engine rather than asserted.

### §3.3, one level up

A word carries a `Derivation`; a sentence carries a `Vec<Construction>`, and it is
produced the same way — **emitted as the generator goes**, never reconstructed from
the output afterwards. Each construction names the syntax parameter that decided it,
so "why is the verb last?" gets the same kind of answer "why does this word start with
/t/?" has had since M3: *because this parameter, from this file*.

A `Slot` stores a `WordId` and a `CognateSetId`, never a rendered string. The surface
is a view; a stored one would desynchronise the first time anything upstream changed
(`docs/adr/0007`), and echoing the descent class is what keeps a sentence comparable
across a family — `BaseRef`'s reasoning, reused.

### Decisions worth knowing

**A proposition names meanings, not words.** `SEE(KING, STAR)` is `ConceptKey`s, which
is exactly why one proposition can go through two languages and the difference be
theirs. A notation over `WordId`s would have made the acceptance impossible to state.

**The notation is about forty lines and shell-safe.** `PREDICATE(ARG, ARG)`, with
`ARG := CONCEPT [":" ADJECTIVE] ["/" POSSESSOR]`. `:` and `/` need no quoting, which
matters for something whose purpose is to be typed at a prompt. There is **no parser
for natural language** and none is planned.

**Case is found by an affix's gloss.** M8 defined an affix's `gloss` as its feature
label ("PL", "PST"), so `ERG` is where the ergative marker lives. A typed `case` field
would need a closed enum of cases compiled in — a taxonomy this project has no reason
to invent before something needs it.

**`compose` is reused, not reimplemented.** A slot is wrapped as a `Morpheme` so
M8's composition kernel lays out the stem and its case suffix. M14 made the same call
from the other direction with `lay_out`; there is still exactly one composer.

**A gap is stated, never faked.** A language with no `ERG` morpheme gets an unmarked
sentence and a printed line saying which morpheme is missing and how to declare it.
A missing *word*, though, is an error: there is no sentence to be had, and coining one
on the spot would be the fabrication this project exists to avoid.

**An unstated parameter falls back visibly.** No word order means SVO — and the
construction record says *"word_order = not stated; fell back to the commonest
order"*, so the fallback is on the record rather than passing as a decision.

### The bug the tests caught

Case marking was written as:

```rust
let mut phrase = phrase?;
let case = case?;          // ← deletes the noun phrase
```

The second `?` returns `None` for the whole closure when there is no case to apply —
so it did not skip the marking, it **dropped the entire noun phrase**. Every language
with `Neutral` or *unstated* alignment produced a sentence containing only its verb.
The default was the broken case, which is the worst possible one to get wrong.

`no_alignment_ever_drops_an_argument` now sweeps all six alignments and asserts the
slot count, because the failing path was the one nobody would think to write a test
for.

### The scope fence

§20.1 names scope explosion as the top risk and this was the milestone most able to
cause it. v0 generates **one clause**: a predicate, up to two arguments, one adjective
and one possessor each. No recursion, no subordination, no coordination, no tense, no
agreement, and **no relative clauses** — even though M17 records a relative-clause
strategy, because recording a parameter and building an engine for it are different
milestones. That ordering was the point of splitting Phase 5 in three.

---

## M17 — Syntax profile · built 2026-08-12 · ✓ verified

Phase 5 opens on the largest remaining gap: you still cannot form a sentence. M17 does
not close it — it makes the grammar **sayable**, so that M18's constructions are built
against something that already validates. **682 tests pass**; clippy clean; fmt clean.

```
$ stemma grammar fixtures/grammar_asterian.ron
Grammar — Asterian (grammar)

  Word order        SOV                           object before verb
  Headedness        head-final                    derived from the orders below, never stored
  Adpositions       postpositions                 *the house in*
  Genitive          genitive-noun                 *the king's road*
  Adjective         noun-adjective                *the stone black*
  Alignment         ergative-absolutive           the one who walks patterns with the one who is hit
  Relative clause   prenominal                    before the noun it modifies
  …
  Typologically harmonic: every stated order agrees with the others.
```

Change one parameter and it says so:

```
    · object-verb order usually goes with postpositions, not prepositions;
      this combination is attested but uncommon

  These describe what is common, not what is correct. A rare language is
  a design; Stemma reports it and does not refuse it (§17).
```

### Decisions worth knowing

**A new crate, `stem_syntax`.** Syntax is not lexicon and not sound change, and Phase
5 is three milestones long — constructions (M18) and syntactic change (M19) both build
on this. It sits above `stem_core` and below `stem_genome`, and depends on nothing
else yet: M17 needs no words to describe a word order.

**Head directionality is derived, never stored.** §7.4 lists it beside the specific
orders and it was tempting to add a thirteenth field. It is a *summary* of the other
four, so storing it makes a second source of truth that disagrees with the first the
moment somebody edits one — the M4 lineage graph's rule (`docs/adr/0008`), applied
one layer up. `headedness()` computes it, and computing it is what makes the harmony
report meaningful rather than a consistency check on redundant data. The sketch prints
it labelled *"derived from the orders below, never stored"* so a reader does not
mistake it for something they can edit.

**Adjective order is excluded from headedness, on purpose.** It is the classic
noun-phrase parameter that does not track the others, and counting it would report
half the world's languages as mixed for a reason that is not about headedness at all.
The reference fixture is head-final everywhere *except* its adjectives, precisely so
this is visible.

**Unspecified is a value, not a missing field.** Every parameter defaults to
`Unspecified` and prints as `—`. A language nobody has given a grammar **says so**
rather than silently claiming to be SVO — `Phonotactics::default()`'s rule, and the
same argument M15 made about a missing word: a gap nothing shows is indistinguishable
from a decision.

**Warnings, never errors — swept.** `no_combination_of_syntactic_parameters_is_ever_an_error`
runs all 960 combinations of the four parameters the harmony checks read, asserts each
one validates, and renders each one. `CLAUDE.md` puts it plainly: *"Resist the urge to
reject a weird language."*

**Two severities, deliberately.** The adposition and genitive correlations are
Warnings; the relative-clause one is a Note. They are not equally strong, and a report
that sounded equally confident about claims of different strength would be misinforming
its reader.

### No fabricated statistics

It would be easy — and it would read authoritatively — to write *"only 4% of languages
do this"* into a harmony message. It would also be unverifiable from inside this
program, which is exactly the false-provenance rule that keeps invented Concepticon ids
out of the concept list. So the messages name the correlation and its direction and
stop, and `no_harmony_message_quotes_a_frequency` greps every message for a digit.

The universals themselves are cited as what they are: Greenberg's word-order universals
(1963), refined by later cross-linguistic work into tendencies rather than laws.

### What was left out, and why

§7.4's list includes **topic/focus marking**, and there is no field for it. Every other
parameter here is a closed choice with an agreed name — a language has postpositions or
it does not. Topic and focus marking is a bundle of interacting facts (morphological
marking, a dedicated position, intonation, and often all three at once) with no
comparable typology to pick from, so any enum shipped now would be an invented
taxonomy rather than a recorded one. It waits for M18, where sentence generation gives
it something to *do* and therefore a shape it has to fit.

That is the same reasoning that kept `WordEntry.ancestor` unshipped at M4 until a
producer needed it: a union with one plausible variant is scaffolding.

### The guards that shaped the code

`#[non_exhaustive]` on every enum meant the renderer in `stem_genome` could not match
on them — a downstream `match` needs a wildcard arm, and a wildcard is what stops a new
variant from being a compile error at the place that has to learn about it. So every
label and gloss moved into `stem_syntax` as a `row()` method, beside the enum it
describes. `PartOfSpeech::name` and `Formation::summary` set that precedent; this is
the third time it has paid.

---

## M16 — Editing in the explorer · built 2026-08-12 · ✓ verified

Phase 4 opens. M11's read-only fence comes down — §20.5 said "before adding full
editing", not "never" — and the rule it was protecting is the reason editing was worth
waiting for. **647 tests pass**; clippy clean; fmt clean.

```
$ stemma set-gloss out/lang.ron w_0002 "the wanderer" --out out/edited.ron
glossed `w_0002` as "the wanderer"
-> out/edited.ron

$ stemma validate out/edited.ron
✓ no issues
```

The same edit in the window produces a **byte-identical file**, because it is the same
call.

### The design, in one sentence

An edit is a **value** (`stem_genome::Edit`) applied by **one function** (`apply`), and
the UI's whole contribution is to build the value and show the answer.

That is not ceremony. A window that mutated a genome directly would be a second
implementation of what an edit *means*, and the two would drift — the CLI refusing an
id collision the window quietly allowed, a file saved from one not matching a file
saved from the other. `a_file_saved_from_the_ui_matches_the_equivalent_cli_command`
runs all four genome edits both ways and compares the bytes.

### Decisions worth knowing

**Validate the clone, then commit — never the other way round.** `apply` works on a
copy, validates it, and returns the original untouched if the edit introduced an
error. A mutate-then-check design leaves a half-applied change behind the first time a
caller ignores a returned report; this one cannot.

**A refusal is "introduced an error", not "has errors".** A file may already be
broken, and refusing every edit to a broken file is how an editor becomes useless
exactly when it is needed. `new_errors(before, after)` compares by `(severity, code,
subject)` as a **multiset** — a *second* `duplicate_word_id` about a different word is
a new fault even though the code already appeared.

**Warnings never refuse anything.** Declaring a concept that shadows a built-in is
odd, reported, and allowed: §17's report-don't-police posture, applied to editing. The
warnings an edit introduced come back in `EditOutcome::introduced` and are shown
beside the change.

**Undo is the file.** Nothing autosaves, nothing is written until Save, and there is
no in-memory history stack — the file on disk is a better undo than any of those and
the user already knows how it works. Unsaved edits are marked `● unsaved` so closing
with changes pending is a visible choice. The CLI has the same contract: without
`--out` it prints what it *would* do and writes nothing.

**Reordering rules is a real edit.** Rule order is chronology and M3 made it
observable — `*taka` gives `tag` under one order and `tak` under another — so
`reordering_a_rule_set_changes_what_the_engine_produces` proves the acceptance is not
vacuous. `move_rule` is a stable rotation, not a swap: moving rule 3 to 0 pushes the
rest down one, which is what dragging a row looks like. A swap would silently reorder
two rules when the user asked about one.

**Identity is not editable.** A `LanguageId` is what `parent` edges, the lineage graph
and every `CognateSetId` scope point at. Renaming a language from an editor would
orphan a family silently, so renaming is a `fork`.

### Three things the existing guards refused

**`stem_genome_never_mints_a_cognate_set` rejected the first draft.** `AddWord` built
a `WordEntry` inline, which meant calling `scoped_cognate_set` from `stem_genome` —
and that source scan bans it, because M4's fork must copy sets and never mint. The
scan was right and the fix is better architecture: minting moved to
`stem_lexicon::authored_word`, beside `build_shaped_lexicon`, `inflect` and `derive`.
The editor now asks the lexicon crate for a word instead of assembling one, and the
scan is exactly as strict as it was before there was an editor.

**The UI guard got stricter, not weaker.** `stem_io::save` left the ban list — that is
the whole of what editing adds — and `::derive`, `build_shaped_lexicon` and
`build_proto_lexicon` joined it. `apply_edit` is deliberately *not* banned; calling it
is the point.

**A doc comment made a claim about a check that did not exist.** `Edit::SetGloss`'s
docs said a paired `gloss_shadowed_by_sense` would explain why setting a gloss on a
drifted word appears to do nothing. It would have — except nothing implemented it. The
trap is real: `display_gloss` prefers a modelled sense over an authored override (M9,
so a drifted meaning cannot be hidden by an inherited label), so glossing `w_0001` of
the reference fixture *stores* the label and *shows* nothing, which from a text box is
indistinguishable from a failed edit. The check now exists as a Note, comes back in
`introduced`, and the window prints it.

### Reading a form back into segments

Adding a word needs the inverse of `Root::written`, so `Root::parse` arrived in
`stem_phonology` — **longest match first**, so a two-character romanisation like `sh`
wins over `s` followed by an unreadable `h`. Greedy without backtracking: a user who
typed something ambiguous wants to be told, not to have a reading guessed for them.

The whole form becomes one syllable whose `pattern` is read off the segments' own
kinds. There is no resyllabifier in this project (unchanged since M3), so inventing
syllable boundaries would be a claim the engine cannot make — and `pattern` is
provenance, never semantics.

An unreadable character reports through `StemmaError::Invalid` with a one-issue report
rather than `NotFound`, because "no {kind} with id `{id}`" is false twice over here:
the text is not an id, and the failure is a position in a string rather than a missed
lookup.

```
$ stemma add-word fixtures/asterian_attested.ron --form tazbo --gloss impossible
Caused by:
    the form `tazbo` is invalid:
      error: [unreadable_form] `zbo` is not written by any phoneme of this
      language (character 3 of `tazbo`)
```

### What the tests do and do not drive

They do **not** drive an `egui` window — there is no headless harness here, and a test
that claimed to click a button while calling a function would be worse than no test.
They drive the whole path the window uses: build an `Edit`, hand it to `apply_edit`,
write with `stem_io::save`. That is the entirety of `App::run` and `App::save_selected`;
everything else in those two functions is a text box and a banner. The window itself
was launched on a real file to confirm it starts and runs.

---

## M15 — Environment & culture profile · built 2026-08-11 · ✓ verified

Phase 3 closes. M13 gave every language 673 meanings by default; M15 is what makes
that a starting point rather than an assertion. A language now says **why** it has
the words it has — and, more importantly, why it has not got the others.
**611 tests pass**; clippy clean; fmt clean.

Two fixtures, one phonology, one concept list, one seed, two ecologies:

```
$ stemma new-lexicon fixtures/desert_asterian.ron
$ stemma new-lexicon fixtures/seafarer_asterian.ron

meaning      desert   island
STAR              1        1     ← the same word, sosem, in both
SAND              4        1
SEA               0        4
FISH              0        3
CATTLE            4        0
ICE               1        0
```

```
$ stemma culture fixtures/seafarer_asterian.ron

    lacks       ICE   — as SNOW, and unlike the desert people these have no
                        northern trade to name it for
```

**`ice` is the inversion that closes the argument.** `DESIGN.md` §7.5 rejected a
larger wordlist partly to avoid forcing a desert language to coin a word for ice. M12
recorded the user's counter — deserts freeze, and you need a name for what the
far-north people live on — and M13 shipped breadth on the strength of it. Here the
*desert* language has the word, because its speakers trade north, and the *tropical
island* language does not. The ecology decides, and either way the decision is on the
record with a reason a reader can disagree with.

### Decisions worth knowing

**No inference engine, and that is the central call.** The tempting design is a
compiled table — "desert ⇒ no SEA, no BOAT, no FISH" — so an author picks a terrain
and gaps appear. That table would be Stemma asserting a great many claims about human
cultures it cannot support, in a program whose premise is that every claim traces to
something a person wrote. There are desert peoples with rich maritime vocabulary
(they trade) and inland peoples with none. So a `CultureTrait` carries **its own
declared consequences**: the author writes "these are high-desert herders, and
*therefore* they lack these twelve meanings", and the engine applies and reports it.
Same relationship a `RuleSet` has to the sound-change engine.

**Distinctions are named, never counted.** `words: 4` would coin four rows all
glossed "sand" — four identical dictionary lines and no information. `senses: [...]`
makes the author state what the four *are*, and the count falls out of the list, so
the data cannot disagree with itself. The desert language's four sands are the kind
that moves overnight, the kind that holds a footprint, the kind that bears a cart,
and the kind that blinds.

**A reason is required, because an unexplained gap is the accident being replaced.**
`Absence::reason` is a `String`, not an `Option<String>`, and an empty one is
reported (`missing_absence_reason`).

**Absence outranks elaboration.** You cannot have four words for a thing you have no
word for; the conflict is reported (`contested_concept`) and resolved toward the
stronger claim, so the report's explanation stays true.

**`stemma culture` on a language with *no* profile says what that silently asserts** —
"this language coins all 673 available meanings, which is itself a claim about its
speakers, just not a deliberate one." Printing nothing would have been the same
invisibility one layer up.

### The draw contract, extended twice — and the bug in the middle

The hard part of this milestone is not shaping the vocabulary, it is shaping it
**without moving a word it did not shape**. Two clauses, and the second one took two
attempts:

1. **An absent concept still draws, and discards.** Skipping the draw would make
   every later word's form depend on how many earlier meanings the profile happened
   to remove, so adding one absence would silently rewrite the rest of the dictionary.
2. **Elaboration's extra words come from `RngDomain::Elaboration`, indexed by the
   concept's own position.**

The first draft of clause 2 used a *single shared* elaboration stream, which is the
obvious reading of "a separate stream" and is wrong. One cursor across all
elaborations means giving `sand` a fourth word moves `water`'s second and third,
because water's draws shifted one position down a stream they shared. Silent, and
exactly the class of rewrite the determinism contract exists to prevent. It was
caught by `elaborating_a_meaning_does_not_move_any_other_word` — a test written
because the property was worth asserting, not because the bug was suspected.

The fix is `stem_core::rng_for_indexed`, which salts the seed expansion with an
index so every position gets its own stream. Its `Option<u64>` salt is deliberate:
`None` hashes byte-identically to the old unsalted expansion, so **no existing stream
moved**, and `Some(0)` is a different stream from `None` rather than conflated with
it.

Together the clauses make a word's form a pure function of its **concept's position**
and the seed. That is what buys the acceptance's legibility: the two fixtures agree on
`star` and every other shared meaning, so every difference between the dictionaries is
attributable to the culture profile and nothing else.

### Reported, never policed

Six checks, all Warnings or Notes: `unknown_environment_concept` (with a spelling
suggestion — a typo'd key means the author's stated gap silently is not one),
`contested_concept`, `duplicate_culture_trait`, `empty_elaboration`,
`missing_absence_reason`, and `large_vocabulary_gap`. A strange culture is a design,
not a fault.

The §17 profile gains a `Vocabulary shaping` band reading the shared
`LARGE_VOCABULARY_GAP`, so band and Note switch at exactly the same count
(`docs/adr/0009`, third instance). `Unshaped` means *no profile declared* — a
different fact from a profile that removes nothing, which reads `Shaped`.

### What is deliberately not here

ROADMAP M15 names four categories; **borrowed-looking is the one not implemented**.
A loanword is not an ecological fact, it is a *contact* fact: making a word look
borrowed means drawing it from a donor language's phonotactics, and with no donor the
only thing on offer is "draw from a deliberately wrong template" — a word that looks
odd for no stated reason, which is precisely the unsupported claim this milestone
exists to remove. It waits for a milestone that can name the donor.

---

## M14 — Derivation · built 2026-08-11 · ✓ verified

M13 made the lexicon big. M14 makes it **made of itself**. 673 roots and 14 authored
patterns produce **3,978 words**, and every coined one records the words it was built
from. **577 tests pass**; clippy clean; fmt clean.

```
$ stemma derive out/deriv_proto.ron --out out/deriv.ron
Asterian (derivation) — 673 base word(s), 14 pattern(s) -> 3305 coined
3978 words over 673 concepts -> out/deriv.ron
```

Then erode the seams, and watch the record outlive them:

```
$ stemma apply-rules out/deriv.ron --rules fixtures/rules_coastal.ron … --out out/deriv_late.ron
$ stemma trace out/deriv_late.ron w_3973

  proto      wikikippa
  │  1  r_velar_lenition    g > ɣ  [2,3)  ·  g > ɣ  [4,5)   → wiɣiɣippa
  │  2  r_gamma_loss        ɣ > ∅  [2,3)  ·  ɣ > ∅  [4,5)   → wiiippa
  │  3  r_apocope           a > ∅  [6,7)                    → wiiipp
  modern     wiiipp

Formation:
  wiki         word     "blood"  →  wii  [w_0009 · cog_asterian_deriv_0009]
  kippa        word     "debt"   →  ipp  [w_0414 · cog_asterian_deriv_0414]

  wikikippa  →  wiiipp   — the seam has eroded; the record above is how the parts
                            are still recoverable
```

`wiiipp` contains neither `wiki` nor `kippa` as a substring. **That is the milestone.**
By eye the word is a monolith; on the record it is still a blood-debt, and each half
is addressable, cognate-visible, and traceable back to the root it came from.

### Decisions worth knowing

**`BaseRef` is a new record, not a reused `MorphemeRef`, and the reason is the
lexicon's size.** A `MorphemeRef` names a `MorphemeId` — an entry in the genome's
*declared* morpheme inventory. A compound's parts are not affixes; they are lexicon
entries, and there are 673 of them. Lifting every word into a parallel morpheme
inventory would duplicate every root's form in a second place (the desync
`docs/adr/0007` forbids) and would bloat every project file by its whole lexicon
again. So `BaseRef` is the word-level twin: same span discipline, same echoed label,
plus the one thing a morpheme has no use for — the base's `CognateSetId`.

**The two record types are complementary, not alternative.** An affixed derivative
records its **base** in `bases` and its **affix** in `morphemes`, because one is a
lexicon entry and the other is a declared morpheme. That is also what distinguishes a
derived lexeme from an inflected cell — both are `WordSource::Derived`, and adding a
variant to re-cut a distinction the record already carries would store a derivable
value, which `WordSource`'s own docs warn against.

**Affixation is productive; compounding is authored.** A derivational affix really
does attach to essentially any word of its class, so applying `-AGT` to all 169 verbs
is *reporting* a fact. Compounding every noun with every noun would coin 123,201
words for a 351-noun language, and whether `star` + `stone` means *meteorite* is a
fact about a culture, not a rule — coining them all would be the tool making a claim
rather than reporting one. That is `DESIGN.md` §7.5's argument applied where it is
actually right, as opposed to where M13 had to overrule it. M15's culture profile is
what will eventually *propose* the pairs.

**`compose` was not copied, it was split.** M8's `compose` and M14's `derive` now
share one `lay_out` kernel, which owns the two things that could silently
desynchronise: the span arithmetic and the rule that every emitted syllable carries
`stress: None` (prosody is assigned once per whole word inside `apply_rules`). Two
composers computing spans separately is a second opinion — and a span that is off by
one does not fail loudly, it misattributes segments in every trace thereafter.

**A derived word mints its own cognate set and names no concept.** `starstone` is a
new lexeme, not a reflex of `star`: sharing its base's descent class would put them
on one row of M5's comparative table, claiming they *are* the same word. And it
realises no built-in concept, because it is on no comparison list — its meaning is
the rendered gloss template.

**Compound pairs are named by `ConceptKey`, never `WordId`.** In a seeded lexicon
`w_0081` is whatever the concept list happened to put at position 81, so a fixture
written against it would have silently meant a different word after M13's append. A
concept key means the same thing in every language, which is what concepts are for.

**`--limit` tightens and never loosens.** A flag typed at the prompt cannot overrun a
bound the file asked for; it takes the min of the two. And it is a cap, never a
sample — a random subset would need an RNG on a path that has none, so it always
takes the first eligible bases in lexicon order.

**Replace, never append — so `derive` is idempotent.** The command drops entries
already marked `source: derived` and re-coins, which is the `new-lexicon` rule.
Running it twice produces byte-identical files instead of doubling the lexicon.

### Three defects the inline review caught

**A `limit` was a shared budget, not a per-pattern cap.** The affixation arm counted
per pattern; the compound arm compared against `coined.len()` — the running total
across *every* pattern. So `stemma derive --limit 2` filled its budget on the
thirteen affix patterns and coined **zero compounds**: no error, no warning, just a
whole formation silently absent from the output. The test that was supposed to cover
it asserted only an upper bound (`coined <= 28`), which passes trivially when a
pattern produces nothing. Both are fixed; the test now asserts the exact count *and*
that a compound is among them.

**`derive` could emit a colliding word id.** Ordinals continue from `lexicon.len()`,
which is collision-free for the sequential ids `build_proto_lexicon` and `inflect`
produce, but not for a hand-authored lexicon with a gap — `[w_0001, w_0004]` has
length 2, so the third coined word would want `w_0004`. `Lexicon::validate` calls a
duplicate id an Error, so the command would have written a file the validator
immediately calls broken while reporting success: the validator and the engine
disagreeing, which is the defect class M1's review established. `derive` now refuses,
naming the id.

**One code, two severities.** `stale_base` covered both a broken cognate-set echo
(a real fault) and a drifted base gloss (entirely ordinary — M9 drift is *supposed*
to move meanings, and the record is deliberately of the meaning at composition time).
Any `warnings().any(|i| i.code == "stale_base")` was therefore ambiguous about which
it had caught. The gloss case is now `base_gloss_drifted`, a Note under its own code.

### The bug caught in the renderer, and why it mattered

The first draft of `render_etymology` printed *"no rule has run over it yet"* whenever
the surface equalled the composition form. That is false for a word that went through
every rule and was moved by none — and it was being printed on real output, because
plenty of compounds survive a rule set untouched. `WordEntry::trace` has carried
exactly this three-state distinction since M3 (`None` / `Some(steps: [])` / `Some`),
with a doc comment explaining that an empty-steps derivation "is the genuinely
different third state". The renderer had collapsed it back to two. Now it says
"exposed to every rule since, and changed by none", and
`a_compound_exposed_to_rules_and_unchanged_says_so_rather_than_claiming_none_ran`
holds the line.

### Fences held

Nothing in `stem_soundchange` changed — the engine still knows nothing about
morphemes, bases, or meaning, and `apply_rules` is still a pure RNG-free function of
five arguments. Erosion of a compound is the *untouched* engine's doing, exactly as
cross-boundary sound change was at M8 (`docs/adr/0010`). `derive` mints through
`scoped_cognate_set` like everything else, and `bases` / `derivations` are additive
`#[serde(default)]` fields, so both reference fixtures round-trip with zero new bytes
(pinned by `a_pre_m14_fixture_round_trips_with_no_new_bytes`).

---

## M13 — A believable core vocabulary · built 2026-08-11 · ✓ verified

The built-in concept list grew from **103 to 673**. A default language is now usable
without the author writing a wordlist first: it can name its kin, its weather, its
livestock, its house, its gods, and can count past two. **540 tests pass**; clippy
clean; fmt clean.

```
$ stemma new-lexicon fixtures/proto_asterian.ron
Proto-Asterian — seed 42 (from the genome), stream `lexicon`
note: 62 entries share a written form with another entry (e.g. a, ap, e, i, inu);
      homophony is real and is reported, not prevented (§17)
673 words over 673 concepts

$ stemma new-lexicon fixtures/proto_asterian.ron --concepts 103 | sha256sum
d16ba86130091d93e3455d2742037b6d199c5181710d15cc28f0f8b9ca508423
```

That second digest is the point of the milestone. `d16ba861…` is the hash **M2
froze** over the 103-word reference lexicon, and it is unchanged — asking for the
same concepts still gives the same words, byte for byte.

The new words are first-class in the engine, not decoration. `ice` — a word §7.5
argued against having at all — forks three ways through the ordinary rule sets:

```
meaning  asterian_attested  coastal  highland  riverine
ice      *kulanpa           kulanp   kulamp    kulampa
father   *etar              edar     edar      etar
```

### Decisions worth knowing

**Append-only, pinned twice, from both ends.** `build` draws one root per concept in
list order from one never-re-seeded stream, so an insertion at index *i* rewrites
every word from *i* onward in every language anyone has ever generated — silently,
with no error and no way to notice. `the_first_hundred_and_three_concepts_are_frozen_in_place`
pins the 103 keys *structurally*, spelled out in full rather than hashed so a
failure names what moved; `the_first_hundred_and_three_words_are_unchanged` pins the
same claim *behaviourally*, by generating and hashing. `PRE_M13_CONCEPT_COUNT = 103`
is the boundary both read.

**Not one new Concepticon id, and that is the honest answer.** All 570 additions use
the `stemma()` constructor and carry `concepticon_id: None`. Mapping 570 meanings
onto Concepticon is real work against a real data file; a plausible-looking integer
under a column bearing an external authority's name is exactly the false provenance
`Concept::concepticon_id`'s docs forbid. It is now *structural*:
`no_concept_added_after_the_first_hundred_and_three_claims_an_anchor` fails on any
`c(…)` below the frozen prefix, so a future session cannot quietly type one in.
Anchoring the stratum later changes no key and no position, therefore no word.

**Organised by field, because a list that grows by association acquires holes where
nobody happened to think.** The 22 semantic fields are Buck's *Dictionary of Selected
Synonyms* / the IDS's — the physical world, kinship, animals, the body, food,
clothing, dwelling, agriculture, physical acts, motion, commerce, space, quantity,
time, perception, emotion, mind, speech, society, war, law, religion — plus a 23rd
for pronouns and function words. The fields are a **checklist against holes, not a
stored property**: `Concept` gained no `semantic_field`, and nothing but the key
reaches disk.

**What Swadesh's list cannot say.** Working the fields turned up gaps that are
invisible until you look for them: no `father` (but `mother`, added at M2), no third
person at all (`I`/`thou`/`we` and stop), no `and`, no number above `two`, no
`house`, no `god`. A language that cannot refer to an absent person or count to three
is not a basic vocabulary, it is a demonstration.

**Two English homographs need two glosses.** `the_gloss_column_is_unique` is what
forces it, and it is right to: `LIGHT_ILLUMINATION` is `light` and `LIGHT_WEIGHT` is
`light (not heavy)`; Swadesh's `FLY_MOVE_THROUGH_AIR` is `fly` so the insect is
`fly (insect)`. Where a distinct English word exists, it beats bracketing — the
"tell an untruth" verb is `DECEIVE`, because `LIE_REST` already owns `lie`.

**No `OMEN`, deliberately.** `fixtures/drift_coastal.ron` declares `sn_omen` as an
unanchored sense a language *invents*, and M9's worked example is `*takala` "star"
drifting to "omen" on one branch. Putting `omen` on the coinable list would blunt the
distinction that example exists to draw.

### The bug M13 exposed, and why it was invisible until now

M12 documented a resolution rule for a project concept whose key collides with a
compiled one — "the compiled meaning wins, so this declaration has no effect" — and
`concept::meanings` did not implement it. It chained unconditionally, so a shadowing
declaration coined a **second** word: two entries, one key, both with the same gloss,
and `by_meaning` returning both. The `shadows_builtin` Warning was describing
behaviour the code did not have.

Nothing hit it at M12, because a collision needed an author to reuse a compiled key
on purpose. Then the compiled list grew and landed on three of
`fixtures/seafarers.ron`'s own declarations — `ICE`, `SAIL`, `OAR` — and the
reference fixture for M12 started emitting three warnings and three duplicate words.
**That is the real shape of this hazard: not an author reusing a key, but the
built-in list growing under a language that already declared the meaning.** It will
happen again at every future append.

`meanings` now drops a shadowed declaration, which is the *reported* resolution and
not a policed one: one key means one meaning or the M5 join is ambiguous, the
compiled meaning wins because it is the one every other language joins to, and
`check_against_concepts` says which declaration went inert and what to do about it.
The fixture drops the three lines and validates `✓ no issues` again — and its comment
records why, because deleting a declaration the compiled list has absorbed is the
ordinary lifecycle of a project concept, not a mistake.

### What this does *not* fix, stated plainly

A project concept sits **after** the built-in block, so when that block grows by 570,
a project language's own words are drawn from a later point in the stream and
re-coining yields **different forms for them**. The built-in prefix is preserved; the
project tail is not. Nothing on disk is stranded — a stored file loads and validates
unchanged, and only re-running `new-lexicon` moves them. Pinning a project tail across
a compiled-list append would need the genome to record the built-in count it was
authored against, and that shim is not worth its weight until someone is actually
stranded by it. It is recorded in `concept::meanings`' docs so the next append does
not rediscover it.

### The reference phonology is now visibly too small, and that is information

62 of 673 words share a written form (~9%), against 10 consonants, 5 vowels and a
CV-heavy template set. Homophony is **reported, not prevented** — rejection sampling
would make the draw count depend on words already produced and destroy the prefix
property that this whole milestone rests on. The note is the tool telling an author
their phonology is undersized for the vocabulary they asked for, which is a fact
about the language and exactly the sort of thing §17 exists to surface.

### The correction, now discharged

M12 recorded the user's counter to `DESIGN.md` §7.5 as the project's position:
deserts freeze, and a language missing words it should have is a worse failure than
one carrying a word its speakers rarely use. M13 is that argument shipped —
`ICE`, `SNOW`, `FROST` and `SEA` are on the list for every language, including the
desert ones. **The deliberate gap is M15's job**: a meaning a culture genuinely
lacks, with a reason on the record, rather than the wordlist's silence.

---

## M12 — Project concepts · built 2026-08-10 · ✓ verified

A language may now declare **its own meanings** alongside the built-in 103, so the
vocabulary ceiling is the language's, not the compiler's. **533 tests pass**; clippy
clean.

```ron
concepts: [
    (key: "TIDE",   gloss: "tide", note: "twice-daily; the whole calendar hangs off it"),
    (key: "KEEL",   gloss: "keel"),
    (key: "ICE",    gloss: "ice",  note: "they trade north; you need a word for what the far people live on"),
],
```

```
$ stemma new-lexicon fixtures/seafarers.ron --seed 7 --out out/sea.ron
114 words over 114 concepts -> out/sea.ron

| maik | /maik/ | ice | noun | `ICE` | `w_0114` | `cog_asterian_seafarers_0114` |
```

### Decisions worth knowing

**The concepts live in the genome, and that is the whole design.** `concept.rs`
compiled the built-in list in precisely so a generated lexicon could not depend on a
sidecar file that `seed`'s contract does not cover — `new-lexicon --seed 42` must not
produce different languages on two machines. A concept carried *inside* the genome
has no such problem: the language is still reproducible from its own file alone. That
is why this is a genome field and **must never become a `--concepts-file` flag**.

**Appended after the built-in list, never interleaved.** The draw contract makes
`build(n)` a strict prefix of `build(n + k)`, so declaring a new meaning cannot move
a word that was already coined — pinned by
`declaring_project_concepts_cannot_change_a_word_already_coined`.

**`ProjectConcept` has no `concepticon_id` field at all.** If a verified mapping
existed the meaning belongs on the built-in list, where every reader gets it. Omitting
the field makes a fabricated anchor *unrepresentable* rather than merely discouraged —
the strongest form of the false-provenance rule.

**`unknown_concept` moved out of `Lexicon::validate`.** From inside a bare lexicon an
invented key and a project-declared one are indistinguishable, so the context-free
method genuinely cannot answer the question — and warning about both would have been
the validator contradicting the generator on every word `new-lexicon` had just
coined. It is now `check_against_concepts(lexicon, project)`, absorbed by the genome
under scope `concepts`, exactly as `check_against_inventory` and
`check_against_semantics` already were, and for the identical reason.

**A project-concept word carries its own gloss**, because nothing compiled can supply
one and `display_gloss` takes no context. That echo has a paired guard
(`concepts.stale_project_gloss`), the same discipline `SenseRef` gets.

**Two more reports, both Warnings:** `shadows_builtin` (a project key reusing a
compiled one — the compiled meaning wins, so the declaration is inert) and the
run-time ceiling note, since `--concepts` can no longer be range-checked at compile
time against a constant that is no longer the ceiling.

### The correction this milestone encodes

`DESIGN.md` §7.5 rejected Swadesh-207 partly to avoid forcing a desert proto-language
to coin a word for ice. That argument is right about *forced* coinage and wrong as a
ceiling — the user's counter is better and is now the project's position: deserts
freeze, and you need a name for what the far-north people live on. **A language
missing words it should have is a worse failure than one carrying a word its speakers
rarely use.** M13–M15 follow from it: ship breadth by default, and make gaps
deliberate rather than accidental.

---

## M11 — Visual explorer · built 2026-08-10 · ✓ verified

The first UI, and the last milestone on the roadmap. `stemma-ui` is a **native
window** — `eframe`/`egui` drawing through wgpu, no WebView, no JS runtime, no Node
in the toolchain, one self-contained 16 MB executable you run from the desktop
(`docs/adr/0013`). **528 workspace tests pass**; clippy clean; the new crate carries
its own guard test.

Four read-only views, each a presentation of a **library** string: **Trace a word**
(§10.2's killer feature — click any word in the list, get its full history),
**Cognate table** (the daughters side by side), **Family**, and **Profile**. Files
open by native dialog, by dropping them on the window, or as argv:

```bash
stemma-ui fixtures/asterian_attested.ron out/coastal_modern.ron out/highland.ron
```

### Decisions worth knowing

**egui over the alternatives, and Tauri ruled out by the constraint.** A system
WebView *is* a browser engine: it would reintroduce HTML/CSS/JS, a Node toolchain,
and rendering that depends on whichever Edge/WebKit the machine happens to have.
Slint would add a **second DSL** to a project that just carefully added one; Iced is
sound but retained-mode ceremony for a viewer that only reads; GTK/Qt would need
system libraries this project has never asked anyone to install. The costs of egui
are recorded honestly in the ADR: a ~16 MB binary and a ~1-minute first build, both
from the graphics stack.

**The UI holds no logic, and a source-scan test enforces it.** Every panel renders a
string from `render_word_history` / `render_cognate_table` / `render_family` /
`render_profile` — the same functions `stemma` the CLI calls.
`the_ui_computes_nothing_it_could_instead_ask_a_library_for` bans `apply_rules`,
`apply_drift`, `scoped_cognate_set`, `parse_rule_set`, `compose`, `inflect` **and
`stem_io::save`** from the crate. That last one is the read-only fence: §20.5's "keep
views read-only before adding full editing" becomes a checked property rather than an
intention.

This is the payoff for keeping `stem_cli` logic-free since M0 and for moving
`render_family` into `stem_genome` at M4 with the note *"the M11 UI must render the
identical text through the identical function"*. **The CLI and the window cannot
disagree about a word's history**, because one renderer sits behind both.

**Monospace is not decoration.** Every renderer aligns columns by character count
through one shared `pad`; a proportional font would undo that work. The window is a
viewer for the CLI's own output, not a reinterpretation of it.

**Verification, stated exactly:** it compiles, launches, holds a real window, and was
run against the Asterian family. It could **not** be screenshotted from inside the
session — the computer-use allowlist resolves *installed* applications and this is a
freshly built dev binary. What the window displays is library output already pinned
by tests elsewhere, but "I have seen it render" is not a claim this entry makes.

**Deferred (§20.5's list is where projects go to die):** every §10.1 view beyond the
four built; all editing and therefore all saving; project files as a concept distinct
from a set of open languages; `.sc` syntax highlighting; theming; window-state
persistence.

---

## M10 — Sound-change DSL · built 2026-08-09 · ✓ verified

Rules become readable. `fixtures/rules_asterian.sc` writes the four M3 sound changes
in §11.1's syntax, and `stemma apply-rules`/`fork --rules`/`rules` accept a `.sc`
wherever they accepted a `.ron`. **528 tests pass**; clippy clean under `-D warnings`.

```text
rule r_0001 "Intervocalic voicing":
  note: "Voiceless stops voice between vowels."
  at: 250
  target: [-sonorant, -continuant, -voice]
  environment: [+syllabic] _ [+syllabic]
  change: set [+voice]
```

**The acceptance is the word "byte-identical", and it is checked as bytes.** Applying
the `.sc` and applying the `.ron` produce files `cmp` reports identical — not
equivalent-looking, identical — and `stemma trace` on the result prints §10.2's chain
`takala → tagala → tagal → taɣal` exactly as the struct-driven pipeline does. That is
what makes the parser a **front end** rather than a second engine (`docs/adr/0012`):
if this syntax could express something the structs could not, or the same thing
differently, the claim would be false.

### Decisions worth knowing

**The parser owns no semantics of its own.** Feature names go straight to
`stem_phonology`'s existing `FeatureBundle::try_from` — there is no spelling the DSL
accepts and the engine does not, because there is only one list. The "did you mean
`syllabic`?" suggestion on a typo comes free from the same place.

**Two acceptance tests, deliberately separate.** One compares the parsed structs to
the deserialised ones; the other compares the evolved output as bytes. If the first
passes and the second fails, the engine is nondeterministic; if the first fails, the
parser is wrong. A single combined test would show the same red for two unrelated
defects.

**Three forced deviations from §11.1's sketch**, each argued in the ADR: a rule
carries an id *and* a name (traces need a stable `RuleId`, humans need a name);
`change: set [+voice]` rather than `voice = true` (the bracket already means "valued
cells", and reusing it makes multi-cell changes fall out); and `copy` exists at all —
§11.1 has no syntax for it, and without it the DSL could not express one of the four
reference rules, putting the byte-identical claim out of reach.

**`V` and `C` are bundle aliases, not letter classes.** §11.1 writes `V _ V`, so the
shorthand ships, expanding to `[+syllabic]`/`[-syllabic]` with a test pinning the
equality. §7.1 is intact: `V` is not "a, e, i, o, u" but a natural class — which is
why a segment a *later rule invents* falls into it automatically.

**The comment marker is `//`, because `#` is the word boundary.** A line-leading
`# note` is indistinguishable from a standalone boundary by any local rule, and every
heuristic separating them makes the lexer depend on the parser — which is how a
comment starts silently changing a rule.

**§20.4's "why did this rule not apply?" diagnostics were already built at M3**, at
three levels: pre-flight, per run, and per word. M10's honest contribution was
finding that refusal reasons rendered with `{:?}` — a reader saw
`Unnameable { bundle: "+syllabic -voice" }`, the shape of the enum rather than the
answer to their question. Now prose, naming the cause and the fix.

**Deferred:** rule scopes (§11.3's "nouns only", "coastal dialect only" — they need
struct fields that do not exist), probabilistic rules, exception patterns, imports
between rule files, a `.ron` → `.sc` emitter, editor tooling.

---

## M9 — Semantics v0 · built 2026-08-07 · ✓ verified

Meaning gets a history. A `semantics` block (senses) attaches to a genome, a
`DriftSet` file carries authored, typed semantic shifts, and `stemma drift` applies
them — recording, per word, exactly which event moved which sense and why.
**509 tests pass**; clippy clean under `-D warnings`; fmt applied.

**The acceptance, run end-to-end.** `stemma drift out/coastal.ron --drift
fixtures/drift_coastal.ron …` turns Coastal's reflex of `*takala` into **"omen"**
through two recorded shifts — a metaphor (`star → divine sign`, priestly, 180y) then
a metonymy (`divine sign → omen, royal sign`, 340y) — while Highland's reflex still
means **"star"**. And `stemma cognates` puts all three on **one row**:

```
meaning  asterian_attested  coastal  coastal_modern  highland
star     *takala            taal     taal "omen"     tagal
```

The form `taal` is *identical* between the two Coastal stages: only the meaning
moved. `stemma trace-word out/coastal_modern.ron omen` now prints §10.2's worked
trace **in full** — the four sound changes, then the two semantic shifts with their
mechanisms and register — which is the first time the program has rendered that
example end to end. `stemma profile` scores `Semantic drift  drifted  (omen: 2)`.

### Review status — panels unavailable on this plan; reviewed inline, 11 defects fixed

**The adversarial panel has never fully run.** Attempt 1: five of nine agents died on
a session limit, *including every verifier*. Attempt 2: five of six died; only the
`simplification` finder completed. Both attempts reported "0 confirmed findings",
and both times that meant **the review did not happen** — not that the code was
clean. Recorded plainly because an unrun review reporting nothing is the most
dangerous kind of green. The account is on a plan
whose limit these panels reliably exhaust, so **the remaining four dimensions
(correctness, constraints, edge cases, tests/docs/scope) were reviewed inline
instead** — by reading the code directly rather than spawning agents. That is the
standing arrangement for this project until the plan changes: panels are a luxury,
the review still happens.

Attempt 1 also revealed a defect *in the review harness itself*: its verify stage
returned `(v && v.verified) || []`, so a dead verifier silently converted its
dimension's findings into an empty list. **19 findings were thrown away.** They were
recovered from the run journal afterwards; the rewritten script now carries raw
findings through regardless of verifier fate.

Between the recovered findings and a direct self-review, **seven real defects were
found and fixed**:

1. **`apply_drift` and `replay` could disagree, so the engine emitted genomes that
   failed its own validation.** The applier filtered acquired senses against `held`
   (survivors ∪ removed) while `replay` filters against the *growing* survivor set.
   Two desyncs followed: `add: ["sn_x","sn_x"]` stored the sense twice but replayed
   once, and a sense named in **both** `remove` and `add` was deleted by the applier
   and restored by replay. Either tripped `semantics.sense_history_desync` — an
   **Error** — on a file the engine had just written, reachable from an ordinary
   authored fixture. Fixed by extracting `advance()`, now **the single definition of
   what a drift step does**, called by both. The agreement is structural, not a
   comment. Two regression tests fail under the old code.
2. **`render_sense_history` resolved a step's event by id, not by its stored index**,
   while `render_derivation` has always resolved by index. `applied_drifts` is a
   *log* whose ids may repeat across strata, so a later step sharing an id rendered
   the earlier event's mechanism, register and date. The accompanying `.filter()`
   was dead logic — its closure ignored the element. Three independent finders and
   the self-review converged on this one.
3. **`grow_family` re-scoped an already-scoped drift report**, so `with_drift`'s
   `drift.*` codes became `drift.drift.*` and, with the branch id,
   `coastal.drift.drift.no_effect`. The `evolve` arm did not re-scope, so the two
   halves of one loop disagreed about who owns the scope. Now merged, not absorbed.
4. **`check_against_semantics` re-implemented `replay`'s fold verbatim** — the same
   divergence class M8's review caught. It now consumes `replay()`.
5. **`target_not_found` was emitted by both the pre-flight check and the applier**,
   printing one fact twice under one code. The pre-flight is now
   `target_not_in_lexicon`; the sound-change precedent keeps its pre-flight and run
   codes disjoint for the same reason.
6. **Padding was hand-rolled five times across `stem_genome`, with two different
   underflow policies.** Three copies used bare `width - len`, safe only under an
   invariant nothing stated or checked — a computed label would panic. Hoisted to
   one `crate::pad`, char-counted and saturating.
7. **The engine's semantics guard had a case-sensitivity hole**: banning `"Semantic"`
   and `"Drift"` capitalized let `check_against_semantics` and
   `check_drift_against_language` straight through — the two functions whose
   appearance would *mean* the engine had started reasoning about meaning. A guard
   that does not guard is worse than none.

Four more from the inline pass:

8. **`sense_history_desync` could not catch the thing it exists for.** It compared
   `replayed.len() == held.len()` plus one-way containment, so `replayed = [A, A]`
   against `held = [A, B]` passed — matching counts, every replayed id present, and
   `B` a sense the word holds with **no recorded provenance at all**. Containment now
   runs both ways.
9. **Half an event could silently do nothing.** `no_effect` fires only when the
   *whole* event is inert, so a removal naming a sense the word never held went
   unreported whenever the same event's `add` succeeded. Now a
   `removal_matched_nothing` Note — "why didn't my change apply here?" is the
   question this project treats as always worth answering.
10. **`CognateCell.drifted` was asymmetric**: with no reference gloss to compare
    against, every other column was annotated as having drifted from nothing.
11. **The generalised anti-fabrication fence had an unguarded escape hatch.** Its
    `looks_like_a_gloss` filter skips anything capitalised or over 24 characters, so
    a fabricated `"Royal Sign"` would pass unexamined — and nothing asserted the
    filter wasn't rejecting *everything*. It now fails loudly if it ever stops
    checking, the same protection the scan itself already had.

Smaller fixes: the dead `fmt_err` kept alive by `#[allow(dead_code)]` (an attribute
suppressing an accurate diagnostic); the `left_no_sense` message naming the wrong
fallback (`display_gloss` prefers an authored override, not the concept); and the
`semantic_drift` band's leading match arm that ignored its own scrutinee.

Verified by hand throughout: the three serde/validation pins hold, and `stemma demo`
and `stemma drift` are each byte-identical across two runs.

### Decisions worth knowing

**Meaning is modelled exactly as form is (`docs/adr/0011`).** `senses` : `sense_history`
:: `phonemic_form` : `trace`, field for field — `input` never rewritten, deltas only,
`replay()`/`final_senses()`, a genome-level `applied_drifts` log beside
`applied_rules`, and the same §16.3 property that the record reconstructs the state.
Nothing new was invented for a job the project already had a mechanism for.

**Drift writes `senses` and nothing else — above all not `cognate_set`.** That is the
milestone: the drifted reflex keeps its row because the comparative table joins by
*ancestry*. M5 shipped `the_cognate_table_joins_by_cognate_set_not_by_meaning` with a
hand-built "omen" daughter to prove the table would survive this; M9 made that
fiction real **without changing its assertion**.

**The engine still does not know what a meaning is.** `evolve`'s signature is
unchanged and `apply_rules` is still a pure RNG-free function of five arguments; a
drifted word survives later sound change because `apply.rs` clones each entry whole.
A second source-scan guard (`the_engine_never_references_semantics`) keeps it that
way, the twin of M8's.

**`display_gloss` gained one prepended tier and shadows, never overwrites.** Sense →
authored override → concept gloss. Drift never touches `glosses`, so `*rekan`'s
authored "king" survives and would return the moment its sense were removed.

**The band measures distance travelled, and says so.** Deliberately named
`SemanticDrift`, not §17's "semantic plausibility": scoring whether `star → omen` is
a *plausible pathway* needs a typology the project does not have, and inventing one
is the fabrication ADR-0009 forbids. `HighlyDrifted` and the
`long_semantic_drift_chain` Note read one shared `LONG_SENSE_CHAIN = 3`; the demo's
own two-step chain sits below it.

**A dishonest test was caught and fixed.** `render_profile`'s `contains("M9")`
assertion would have kept passing on `ScriptHistoryCoherence`'s `"M9+"` after the
semantic row was filled. The deferred dimensions now name design *sections* (`§7.6`,
`§18`) and the assertion is inverted.

**The demo's anti-fabrication fence was rewritten stronger, not retired.** `tazal`
and `night-signal` stay banned forever; `omen` is allowed only when a real event
produced it, the mechanism is named, and the closer stops promising what shipped. A
new general guard replaces the string list with the rule it stood for: every gloss
printed must be a concept gloss or a declared sense.

**Scope fenced (§20.1).** No LLM or probabilistic drift — v0 *applies* authored
drift as M3 applies authored rules. No syntax, no script history, no polysemy graph,
no sociolinguistics beyond a free-text register label, no `HistoricalEvent` union
(it would renumber every stored `RuleApplication.index`). `EventId`, declared at M0
and unused since, finally has a producer.

---

## M8 — Morphology v0 · built 2026-08-01 · ✓ verified

Stemma grows morphemes. A `morphology` block (stems, affixes, paradigms) attaches to
a genome; `stemma inflect --paradigm NUMBER` materialises the paradigm's cells as
ordinary `WordEntry`s (the regular, pre-sound-change forms); `apply-rules` evolves
them; `stemma paradigm` renders the result. **457 tests pass**; clippy clean under
`-D warnings`; fmt applied.

**The acceptance, run end-to-end:** `inflect fixtures/morphology_asterian.ron
--paradigm NUMBER` gives a regular `-ka` plural (`tiraka, menaka, tanka, sulka`).
After `apply-rules … rules_intervocalic_voicing.ron`, `stemma paradigm` shows the
suffix split into **two allomorphs** — `-ɡa` after the vowel-final stems (`tiraɡa`,
`menaɡa`), `-ka` after the consonant-final ones (`tanka`, `sulka`) — with each cell
naming the rule that fired or "did not apply". `stemma trace w_0002` shows the
`r_ivv` step voicing `k > ɡ` at `[4,5)` in `a _ a`; `stemma trace w_0006` shows
`r_ivv … — did not apply`. `stemma profile` scores `Morphological irregularity
allomorphic (PL: 2)`. That is a regular paradigm made irregular *purely* by an
ordered sound change, with the trace explaining why.

### Decisions worth knowing

**No new crate; the engine does not change (`docs/adr/0010`).** A morpheme's `form`
is a `Root`; `compose` concatenates syllable lists, so a morpheme boundary is a
segment adjacency and the engine's cross-boundary environment scan gives conditioned
allomorphy for free. An inflected cell is an ordinary `WordEntry`, so `apply-rules`,
`fork`, `trace`, `cognates`, and export all work on it unchanged — `apply.rs` stays
pure and RNG-free. A source-scan guard test (`the_engine_never_references_morphology`)
catches any morphology type or operation creeping into `stem_soundchange`.

**The composition record is spans, not surface segments.** `WordEntry.morphemes`
stores a `MorphemeRef { morpheme, role, gloss, start, end }` per morpheme — the flat
span in the composition form (= `Derivation.input`, which never changes). The surface
allomorph is recovered by `Derivation::surface_of_input_span`, which replays the
trace carrying each segment's origin index. Storing surface segments would be the
`docs/adr/0007` desync. This discharges §3.3 for composed forms: a composed form with
empty `morphemes` is a form with no recorded composition.

**Each (stem, cell) mints its own cognate set.** `inflect` calls `scoped_cognate_set`
(the sole mint site; `morpheme.rs` joined the source-scan). `tira-SG` and `tira-PL`
are different entries, so different sets — and `fork` copies each verbatim, so two
daughters' `tira-PL` stay cognate while SG ≠ PL (`docs/adr/0007`).

**Irregularity is measured and joins the profile the M7 way (`docs/adr/0009`).**
`morphological_irregularity` counts each affix's distinct surface allomorphs; it
fills M7's `NotModelled::MorphologicalIrregularity`, which leaves the deferred list
and becomes a scored `MorphologicalIrregularity` band. The `HighlyAllomorphic` band
and the `high_morphological_irregularity` validation **Note** read one shared
`HIGH_ALLOMORPH_COUNT` (= 3), so they agree by construction — a projection test pins
it. The demo's two-way split is `Allomorphic`, below the Note. Never an Error (§17).
`high_change_density` stays as a distinct coarse signal; only its *claim* to stand in
for morphological irregularity is retired.

**One deviation from `M8-SPEC` §3:** `compose` takes `&[&Morpheme]` and reads each
affix's own `role`, not the spec's redundant `&[(&Morpheme, MorphemeRole)]` tuple —
the stored role is authoritative and the tuple could contradict it.

**Scope fenced hard (§20.1).** v0 is concatenative (prefix\* · stem · suffix\*) and
nothing else: no non-concatenative or fusional exponence, no grammaticalization, no
typed feature system, no typological profile, no syntax, no resyllabifier, no
semantics, no paradigm export/UI, no append-inflection. All named in `docs/adr/0010`.

---

## M7 — Plausibility profile · built 2026-07-23 · ✓ verified · **Phase 2 begins**

The validator learns typology. `stemma profile` prints DESIGN §17's scored
dimensions — typological rarity, phonotactic complexity, historical depth — as
qualitative *bands*, and the graded report now carries specific, non-authoritarian
plausibility warnings (an 80-consonant inventory, a three-consonant cluster, a
rapid change history). **427 tests pass**; clippy clean under `-D warnings`. This
opens Phase 2 (depth) — and it is the *same* `ValidationReport` with more checks,
not a new subsystem.

Verified by running it: `stemma profile fixtures/proto_asterian.ron` reports
`typical / simple / none`, names the four unbuilt dimensions as "not yet modelled",
and stays quiet (0 warnings); `stemma profile fixtures/implausible_clusters.ron`
reports `complex` and fires `large_consonant_cluster` — while still validating.

### Decisions worth knowing

**The scored block is a derived read-model, not a second validator
(`docs/adr/0009`).** The plausibility *warnings* are ordinary `report.warn`/`note`
calls in the existing `Validate` impls — they reach `stemma validate` with zero new
plumbing, which is the whole proof this is "more checks on the same report". The
*bands* are a pure `fn(&genome) -> PlausibilityProfile` presented alongside the
report the way `summary()` already is: zero `Issue`s, no `Severity`, no serde,
never stored.

**The band and the warning read one shared set of constants, so they cannot
disagree.** `LARGE_CONSONANT_COUNT`/`LARGE_VOWEL_COUNT`/`LOPSIDED_RATIO`/
`VERY_SMALL_TOTAL` live once in `stem_phonology`; both the size warnings and
`PhonemeInventory::rarity()` read them, so the `Rare` band holds *exactly when* a
size warning fires — a test pins the projection. That is the line that keeps the
descriptive view from drifting into a parallel check registry.

**Honesty over completeness.** §17 lists seven dimensions; M7 has real data for
phonology and coarse lineage only. The five that need unbuilt milestones render as
explicit "not yet modelled → Mn" lines, never a number; §17's composite "82%" is
dropped (any percentage would overclaim or average over dimensions that do not
exist); syntax/word-order is dropped entirely, with no "not modelled" line, so it
does not invite a syntax engine (§20.1). The lineage signals say out loud that they
count *authored rules* (an editorial granularity) and read only the sound-change
log — not the morphological irregularity §17's third example is really about, which
waits for M8.

**Report, do not police (§17).** Every new check is a Warning or Note — an
80-consonant monster earns warnings and still `validate()`s. No new Error; the
acceptance suite pins `is_ok()` on the weird languages. The deliberate *absence* of
a small-vowel warning (a two-vowel system is attested) is the same call.

**One float left the codebase.** The legacy `lopsided_inventory` used an `f32`
ratio in its message; it is now integer cross-multiplication (`c > RATIO * v`), so
the last float leaves the validation control path and the constant is shared with
the band.

### Adversarial review — 6 findings, all fixed

A five-dimension panel (17 agents) reviewed the implementation, each finding
independently reproduced. Six distinct defects, all minor, all fixed — and the two
that mattered were both about the projection invariant the design leans on:

1. **The empty inventory broke `Rare ⟺ a size warning fired`.** `rarity()` scored a
   zero-phoneme inventory `Rare` (via `c == 0` / `total < 5`), but `validate`
   early-returns on an empty inventory with only the `empty` Error — so *no* size
   warning fires, and the band disagreed with the checks it is documented to
   summarise. Fixed with an explicit empty guard (empty → `Typical`, mirroring
   `validate`'s early return), and the projection test now sweeps the boundaries
   (45/46 C, empty, vowelless) in both directions.
2. **The `rarity()` doc claimed a vowelless inventory "counts as Rare"** — but the
   code (correctly) scores it `Typical` (a vowelless inventory trips only the
   `no_nucleus` Error, not a size code). The doc was the wrong half and pointed
   *opposite* the invariant; corrected.
3. A doc said "five" not-modelled dimensions; `NOT_MODELLED` holds **four**. Fixed.
4. **A botched doc merge** — the M7 insertion had split the `Phonotactics` struct
   doc mid-sentence, orphaning "The genome field stays". Repaired; the const/enum
   now sit above the struct with intact docs.
5. The depth line rendered "over 1 years" — `years` is now singularized.
6. The `implausible_clusters.ron` header still carried the whole copy-pasted
   Proto-Asterian comment block; removed.

One finding was refuted (the closing "bands sit against attested ranges" line vs
the coarse historical-depth band — a wording nuance the skeptic did not consider a
real defect). Net at **427 tests**.

### Gotchas for the next session (M8 — morphology)

- **M8 fills the first "not yet modelled" row.** `NotModelled::MorphologicalIrregularity`
  (M8) is the honest placeholder; when morphemes exist, M8 measures real
  irregularity and the profile gains a dimension. `high_change_density` is the
  *coarse* stopgap that reads only the sound-change log — M8's measure supersedes
  it, not replaces the report check.
- **Add profile bands the way M7 did: shared constants, a projection test, no
  float, no fabricated score.** The band must agree with a report check or it is a
  second opinion (`docs/adr/0009`).
- **`ipa_not_nfc` is deferred, not dropped** — the `by_ipa` comment now points at a
  later data-hygiene pass (it is interchange hygiene, not a §17 typological
  dimension, and would pull in a Unicode-normalization dependency).
- **`reference_phonology.rs` pins the proto's codes to `["lexicon.empty"]`** — a
  new check that fires on Proto-Asterian breaks it. That guardrail is load-bearing;
  keep new plausibility checks quiet on the reference family.

---

## M6 — The portfolio demo · built 2026-07-22 · ✓ verified · **Phase 1 complete**

`stemma demo` tells the whole story in one command: it grows the Asterian family
from the committed fixtures, builds the comparative table, traces five words in
full, and writes it all as a self-contained Markdown document — "Growing a
Language Family in 90 Seconds." **404 tests pass**; clippy clean under
`-D warnings`. This is the last milestone of Phase 1: the diachronic kernel
(inventory → generation → sound change → forking → the comparative views) now
runs end to end from the command line.

Verified by running the ROADMAP acceptance: `stemma demo --out output/demo.md`
writes a 199-line document (proto glossary, three daughters with their rule
histories, the cognate table with the `star` row `*takala | taal | tagal | tala`,
and five fenced etymologies from `*takala → taal` to `*mikala → miala`); two runs
are byte-identical; and a `stem_export` golden pins the exact bytes.

### Decisions worth knowing

**M6 was composition, and stayed composition.** No engine work: every genome comes
from `evolve`, every form from `written`, every derivation from
`render_derivation`, every table cell from `cognate_table`. The only new *logic*
is a `stem_genome::grow_family` helper (pure `evolve` + `assemble`) and one new
renderer.

**Build was split from render — the panel's improvement over all three proposals.**
Each proposal put the whole demo in one `stem_export` function that called
`evolve` internally. That would have put engine-*build* code in the render crate
(an ADR-0006 stretch) **and** made the renderer's canary engine-dependent — any
`apply.rs` change would move it, defeating the M1 canary-vs-golden isolation.
Instead `grow_family` (build) lives in `stem_genome` and `write_family_demo`
(render) is a *pure* projection over an already-built graph, so its canary is a
true renderer-only tripwire: it hand-builds a graph with a hand-authored
`Derivation`, runs no engine, and no fixture or `apply.rs` change can move its
bytes.

**`stem_export` gained a direct `stem_soundchange` dependency** — a legal downward
edge (it was already transitive via `stem_genome`), needed because the trace
blocks call `render_derivation`. Rendering a derivation is rendering, so ADR-0006
holds; no new ADR.

**The demo is honest.** DESIGN §21's flashiest steps need unbuilt milestones, so
the demo does not fake them: it shows Highland `tagal` (the real form; §21's
`tazal` is an unreachable `g→z` place shift), and the closer *names* meaning drift,
morphology, and a visual explorer as forthcoming without printing a drifted gloss
or an "omen" token. Two tests (a `stem_export` golden assertion and a CLI
acceptance) fail if `tazal`/`omen`/`royal sign`/`night-signal` ever appear.

**Determinism is by construction.** No RNG (the proto lexicon is authored, not
generated), no clock (the colophon is dateless; the source-scan now bans
`SystemTime`/`Instant`/`chrono::`), no map, no sort, `include_str!` inputs so the
binary runs identically from any directory. Two runs are byte-identical, pinned at
both the library and the binary.

**The proto roster is bespoke, not `write_lexicon_markdown`.** That renderer emits
its own H1 and prose that is *false* for the authored fixture ("coined … on the
`lexicon` RNG stream at seed N" — the nine words were authored, not coined). The
demo's compact `Gloss | Form | IPA` roster keeps one H1 and one honest provenance
line. Recorded here so the second rendering surface is tracked, not smuggled.

### Adversarial review — 3 findings, all fixed

A five-dimension adversarial panel (scope/anti-fabrication, determinism,
renderers, architecture, canary-vs-golden) reviewed the implementation, each
finding independently reproduced. Three distinct defects, all fixed:

1. **The family-demo canary pinned no bytes (major).**
   `the_family_demo_canary_matches_its_frozen_bytes` only did `contains()`
   landmark checks despite its name, so a genuine `write_family_demo` regression
   (a changed rule-bullet format, a dropped blank line) would slip past it and be
   caught only by the *re-baselineable* golden — exactly the "a renderer
   regression must not hide" failure the discipline exists to prevent. Now a real
   `assert_eq!` against ~1.6KB of inline expected bytes, engine-independent (the
   canary hand-authors a `Derivation`, so no `apply.rs` change can move it) — a
   true renderer-only tripwire, like the cognate-table canary.
2. **The demo printed a non-runnable command (minor).** The proto section said
   "one command away: `stemma export-md`" — but `export-md` requires a `<PATH>`,
   so a reader copying it hit exit 2. Now `stemma export-md <file>`.
3. **The cognate-table notes branch had no byte coverage (minor).** The canary
   had empty notes; added a test rendering notes as escaped italic bullets.

Also fixed a *refuted-but-real* latent issue: `write_family_demo`'s daughter
heading printed the daughter's absolute depth as "+Ny from proto" — correct only
because the demo's proto is at depth 0. Now the edge delta
(`daughter.depth − proto.depth`), correct for any proto; the demo bytes are
unchanged.

### Gotchas for the next session (Phase 2 begins)

- **Phase 1 is the MVP and it is done.** M7 opens Phase 2 (depth): grow the M0
  validation report into §17's typological *plausibility profile* — scored
  dimensions and specific, non-authoritarian warnings. It is the same
  `ValidationReport` with more checks behind it, **not a separate subsystem**, and
  §17's rule stands: report, do not police. Read `DESIGN.md` §17.
- **The demo's story is single-sourced in `write_asterian_demo`.** The CLI, the
  golden test, and (eventually) the M11 UI all call it, so the document cannot
  drift between front ends. Change the story there, re-baseline
  `golden/family_demo.md`, and only after the inline canaries stay green.
- **`grow_family` is the sanctioned way to build a family in code.** M11 will use
  it too. It returns a report per daughter, parallel to the specs; the demo merges
  them to stderr.
- **`escape`/`fmt_err` are now `pub(crate)` in `markdown.rs`.** The cognate-table
  and demo renderers share them; the dictionary golden is unchanged.

---

## M5 — Cognate tables & word traces · built 2026-07-21 · ✓ verified

The family becomes *legible*. `stemma cognates` prints §10.3's comparative table —
reflexes of each meaning across every daughter, side by side — and `stemma
trace-word` prints §10.2's full derivation addressed by meaning instead of by word
id. **385 tests pass**; clippy clean under `-D warnings`.

Verified by running the ROADMAP acceptance: `stemma cognates … --meanings water
sun star king mother` prints the table with `star → *takala / taal / tagal /
tala`, `king → *rekan / rean / regan / rean`, and `mother → *mikala / mial / migal
/ miala`; `stemma trace-word out/coastal.ron star` is byte-identical to `stemma
trace out/coastal.ron w_0001`.

### Decisions worth knowing

**The whole milestone was one lexicon query, one graph view, two thin verbs, and
one fixture word.** Everything else was already in the tree: the cognate set is
the cross-language join (`docs/adr/0007`), and `render_derivation` already emits
the entire §10.2 ledger. M5 changed no engine code and no file format.

**Meaning resolves by *displayed gloss*, never by concept.** `Lexicon::by_meaning`
matches `display_gloss()` case-insensitively, so `king` finds `*rekan` — concept
MAN, gloss override "king" — which a concept-key match (`king → KING concept →`
no fixture word) would render as an empty row. The fixture is built to expose
exactly that bug: `w_0005` is deliberately concept MAN with a "king" gloss (the
etymology-vs-surface seed M9 needs). One shared resolver serves both `cognates`
and `trace-word`, so the two can never disagree about what a meaning names.

**The table joins by cognate set, resolved *once* against the reference.** Not
re-resolved by meaning per column — that would be a concept join, and under M9's
meaning drift a daughter whose reflex shifted sense would silently drop out of its
own row. Ancestry gets a language into the row; the displayed meaning is only the
row label. A daughter lacking the set is a gap (`—`), not an error (plausibility
reported, not enforced). Each cell renders against its *own* column's inventory,
because Highland's `tagal` carries `ph_g`, absent from the proto inventory —
rendering with the reference's would abort the whole table.

**`MOTHER *mikala` earned its place with a real contrast.** The velar chain again,
but `/i/` is `[-low]`, so Riverine's low-vowel coalescence cannot fire and it
alone keeps all three vowels (`miala`), where STAR's `*takala` coalesced to
`tala` — a lesson (coalescence is conditioned) no earlier word teaches. Chosen
over a zero-risk `*maka` (which would have been isomorphic to `*taka`). The
reflexes were **verified against the engine**, not hand-computed into the goldens.

**`stemma trace-word coastal star` is file-native.** ROADMAP writes a language id;
the CLI is file-native (there is no language registry — the lineage is derived
from files), so "coastal" is `out/coastal.ron`, the same deviation `trace` and
`family` already made. `trace-word` is `trace` with meaning-addressing swapped for
id-addressing: same renderer, zero new rendering.

### Adversarial review — clean

A five-dimension adversarial panel (resolver correctness, the cognate-set join,
determinism/rendering, CLI wiring & fixture correctness, placement & invariants),
each dimension charged to reproduce any defect against the code, returned **zero
findings**. Credible for the smallest milestone of the phase — one lexicon query
and one graph view over established patterns — and backed by front-loaded
verification: the `*mikala` reflexes were confirmed against the engine before the
goldens were written, the acceptance table's exact bytes were baselined from the
real renderer, and `trace-word star == trace w_0001` was checked byte-for-byte.

### Gotchas for the next session

- **The reference fixture is now nine words.** Adding `w_0009` MOTHER moved five
  M4 test goldens (`forms()` `1..=9`, `len()==9`, the reflex table's ninth row,
  coverage `9/9`, the family snapshot's `9 words`) and three doc pins — all
  updated. The M4 PROGRESS entry above now reads `9/9` and `27-cell` for accuracy
  (a reader running the M4 tests today sees those numbers). `proto_asterian.ron`
  is untouched (its own `w_0009` is BLOOD); M3 acceptance stays green because
  `*mikala` mints nothing new.
- **`render_cognate_table` pads by char count, not byte length**, so `ŋ`/`ɣ`
  cells align. Its output is the M6 demo's raw material and will want a
  `stem_export` Markdown/CLDF projection over the same `CognateTable` struct —
  the struct carries the ids and names a projection or a clickable cell needs.
- **`cognates` writes only the table to stdout;** the reference banner and any
  notes go to stderr, so the table stays diffable (the `generate-roots` split).
- **The M5 continuity debt is settled, not carried forward:** MOTHER and KING were
  on the concept list since M2 (a prior session's hook for exactly this); MOTHER
  is now a real word, and `king` resolves by gloss.

---

## M4 — Forking & lineage · built 2026-07-21 · ✓ verified

Languages now *branch*. `stemma fork` splits a parent into a daughter — a
verbatim copy under a new identity, or (with `--rules`) a daughter that has
already undergone its own sound changes — and `stemma family` assembles several
language files into a lineage, printing the family tree, cognate coverage, and a
graded report. **360 tests pass**; clippy clean under `-D warnings`.

Verified by running the ROADMAP acceptance end to end: forking
`asterian_attested` three ways under three hand-written rule histories yields
Coastal, Highland, and Riverine, whose lexicons differ pairwise — `*takala`
reflects as **taal** / **tagal** / **tala**, and the whole 27-cell golden table
matches the engine cell for cell. `stemma family` reports **9/9 cognate sets
present in all three** daughters, every daughter's trace replays to its stored
form, `stemma trace out/coastal.ron w_0001` walks unbroken from *takala to taal,
and two fork runs write byte-identical files.

### Decisions worth knowing

**The lineage graph is derived, never stored.** `DESIGN.md` §8.6 sketches a
`HashMap` of nodes plus a stored `Vec<LineageEdge>`; both were rejected.
`LineageGraph` holds a `Vec<LanguageGenome>` in argv order and derives every edge
from the `parent` field on demand — a stored edge beside `parent` is a second
copy of one fact that nothing keeps synchronised, the exact desync class this
project has refused three times (`form`, stored intermediate forms,
`Syllable::pattern`-as-semantics). And there is **no map anywhere**: a `HashMap`
leaks iteration order toward output (§9.4) and swallows the duplicate ids the
validator must see. Tens of nodes, linear scans, fully deterministic.
(`docs/adr/0008`.)

**`fork` is identity-plus-split; `evolve` still runs the rules.**
`LanguageGenome::fork` clones the genome verbatim and relabels it — no rules, no
RNG, no form change. The CLI's `fork --rules` calls `evolve` through the *same*
load→gate→write helper `apply-rules` uses, so the write gate (refuse on
validation Errors) cannot drift between the two verbs. Rejected outright: an
in-place `advance` operation, because `apply-rules x --out x` is not idempotent —
re-running one line applies a stratum twice, the double-application hazard
`applied_rules`' past-tense contract exists to forbid.

**The cognate obligation was one line, and it is discharged by construction.**
`fork` clones the lexicon whole, so every `cognate_set` is byte-identical and no
code path can mint. A new source scan (`stem_genome_never_mints_a_cognate_set`,
walking `src/*.rs` at runtime) proves it, and it closed a real gap: the old
`stem_lexicon` scan used `include_str!`, which cannot cross a crate boundary, so
it never saw `stem_genome` and already missed `stem_lexicon`'s own `trace.rs`
(now added). The rule is unchanged; its enforcement is honestly per-crate.

**No `LineageEdgeKind`, not even `Descent`.** With edges derived there is no file
format to stabilise, so a one-variant enum is scaffolding by the `HistoricalEvent`
precedent. A dialect split *is* descent; "split" is out-degree, derivable. The
four contact-like kinds are not derivable from `parent` (a contact edge is a
second parent) and arrive with their producers in M7+.

**`WordEntry.ancestor` stays unshipped.** ADR-0007 deferred it "to M4"; M4
discharges it instead. Both fork and evolve copy word ids verbatim, so a
daughter word's ancestor is *always* the same-id parent entry — a stored field
would hold a tautology. It ships when a producer writes a non-identity value (M7
borrowing, M8 derivation); until then a library test plus the cross-file
`family.word_id_orphan` Warning defend the derivability.

**`chronology_years` is absolute from the lineage root.** Already the codebase's
commitment (M3 dated its last rule 480 and passed `--years 480`; the monotonicity
check runs over the whole concatenated log). So the three daughters' rule dates
are absolute, and `--years` conventionally equals the last rule's date. Family
edges report the depth *delta* (`+470y`), never the total; a negative delta is
`family.depth_regression`, an Error.

### The fixtures — three crossing isoglosses

Chosen so the acceptance contrasts are real, not decorative. Coastal innovates a
feeding chain (voicing → lenition → γ-loss) plus apocope; Highland shares voicing
and apocope but assimilates nasals instead of leniting; Riverine shares only the
nasal isogloss and keeps its final vowels and voiceless stops. Result: voicing
{Coastal, Highland}, apocope {Coastal, Highland}, nasal assimilation {Highland,
Riverine} — three isoglosses, each shared by a different pair, wave-model in
miniature. Two convergences fall out (`rean` and `ta` are identical in Coastal
and Riverine through different histories, distinguishable only by trace), and the
cross-file rule-id convention is honest: `r_ivv` is byte-identical in Coastal and
Highland, only its per-branch date differs.

### Adversarial review — 3 findings, all fixed

A five-dimension adversarial panel reviewed the implementation (each finding
independently reproduced by a skeptic before it counted). It surfaced three
distinct defects, all minor, all in the family-coverage rendering, now fixed at
**361 tests**:

1. **Gap language lists came out in DFS order, not node order.**
   `descendant_indices` walked the closure with a stack (`frontier.pop()`, so
   depth-first despite a comment claiming breadth-first), and that order *did*
   reach output — the `gap: X absent from …` line `stemma family` prints. A
   comment even asserted it "does not reach output", which was false. The closure
   is now sorted into stored order before it returns, honouring the documented
   "languages in node order" contract for any tree shape. Caught only because the
   acceptance family is flat (one root, three direct children), where DFS order
   coincides with node order — a depth-≥2 family exposed it.
2. **`apply-rules --years` lacked the `range(0..)` guard `fork` has.** The M4
   refactor routes both verbs through one write gate, but only `fork` rejected
   negatives; `apply-rules --years=-100` on a deep parent would persist a
   descendant *earlier* than its parent (total depth still positive, so the
   genome's `negative_lineage_depth` Error never fires). Both verbs now carry the
   same guard, with a test asserting it.
3. **The coverage line printed "1 sets".** The set count was the one count in
   `render_family` never singularised. Fixed.

One finding was **refuted**: the mint-scan's test-region cutoff (it stops at the
first `#[cfg(test)]`) and its non-recursive `read_dir` are real limitations, but
the skeptic confirmed the current flat `stem_genome/src` is clean, so it is a
latent-robustness note rather than an M4 defect — left as-is.

### Gotchas for the next session

- **`stemma family` output is two parts.** `render_family` (tree + coverage) is
  library-owned and snapshot-pinned for M6; the validation report prints
  *separately* via `print_report`. The snapshot covers only the first part.
- **The acceptance family is valid but not silent.** Each daughter carries
  `lexicon.syllable_shape_mismatch` **Notes** (stale `Syllable::pattern` after
  deletions — M3 established this is Note-severity on a word with a derivation),
  so the family report honestly ends `✓ valid — 0 warning(s), nothing blocking`,
  never `✓ no issues`.
- **Deviation from ROADMAP's literal "Proto-Asterian":** the fork parent is
  `asterian_attested.ron`, the proto stage that *has* words (`proto_asterian.ron`
  has none, and M3's own acceptance forked it). Recorded in the fixture headers.
- **Deviation from DESIGN §21's sketch:** Highland gives `tagal`, not §21's
  `tazal` — ɡ → z is a place shift `set` cannot express (`overlay` never removes
  a cell, apply.rs). Revisit at M10 if node-level writes arrive.
- **M5 continuity debt (recorded, not solved):** ROADMAP M5 names meaning MOTHER
  and gloss "king", but the fixture has no MOTHER concept and "king" is a gloss
  override on MAN. M5 appends a MOTHER word (checking no golden pins the word
  count) or adjusts its meanings.

---

## M3 — Sound-change engine · built 2026-07-20 · ✓ verified

The heart of the program. Languages now *change*: `stemma apply-rules` runs an
ordered rule sequence over a lexicon, every application writes a per-word
`Derivation`, and `stemma trace` prints §10.2's killer feature — the full causal
history of a word, rule by rule, back to the proto-form. **314 tests pass**;
clippy clean under `-D warnings`.

Verified by running it: applying `fixtures/rules_asterian.ron` to
`fixtures/asterian_attested.ron` reproduces the design doc's own worked example
exactly — `takala → tagala → tagal → taɣal` — and the other seven fixture words
land on their hand-computed forms (`tag`, `sawel` unchanged, `akw`, `reɣan`,
`saŋk`, `amp`, `ant`). Three phonemes were minted along the way (/ɡ/ /ɣ/ /ŋ/),
two runs write byte-identical files, and the M1/M2 corpus digests are untouched.

### Decisions worth knowing

**The engine uses no RNG at all.** `apply_rules` is a pure function of its five
arguments. `RngDomain` gained no variant — the strongest determinism claim the
project can make, and it was free.

**A rule can create a phoneme the language does not have.** Voicing /k/ yields a
bundle no inventory phoneme carries; a compiled-in 20-row reference table
(`stem_phonology::reference`) names it. The minted /ɡ/ is **U+0261 SCRIPT G**,
not ASCII `g` — the ASCII letter is its romanisation, which is why `written()`
prints ROADMAP's literal `tagala` while `ipa()` prints `taɡala`. Ids come from
the table row (`ph_g`), so two sister languages independently innovating /ɡ/ get
the *same* phoneme — a correct merger, not a collision. Exact match at every
tier: on this fixture `/k/[+voice]` is Hamming-1 from /k/ itself, so any fuzzy
resolver would silently undo the rule and write a trace that lies.

**Simultaneous application over a frozen snapshot.** Within one rule, every
match is found against the word as it stood before the rule; a rule's output is
never visible to its own matching, and a `Copy`'s donor reads from the snapshot
too. With single-segment targets, application is provably commutative over the
site set — which is why there is no `ltr`/`rtl` flag and no overlap resolver:
they would be unobservable settings, i.e. lies about what the file format means.
The multi-segment tie-breakers are pre-committed in `apply.rs`'s module doc.

**The design doc's own example cannot demonstrate rule ordering.** Verified
arithmetically: `takala` yields `tagal` under either order of voicing and
apocope, because its final vowel never conditioned the /k/. The fixture ships
`*taka`, which genuinely bleeds (`tak`) and counterbleeds (`tag`) — and a test
pins the *reason* it exists so a future session cannot delete it as redundant.

**Assimilation is a feature copy that can transfer absence.** One rule (`copy
place from after(0)`) resolves three ways on the fixture: /n/+/p/ → declared
/m/; /m/+/t/ → declared /n/ — which requires the copy to *remove* the rounding
cell /t/ leaves absent, the reason `FeatureBundle::unset` exists; /n/+/k/ →
minted /ŋ/. The node is the unit of copy (`FeatureNode::Place` carries the
articulators *and* their dependents), making the ill-formed partial copy
unrepresentable.

**Stress landed as a syllable-scoped store, not a feature.** `Prosody::assign`
marks a word once in its life (all-or-nothing, so splitting a rule sequence
across two runs gives the same language), and `Some(Unstressed)` never matches
an unmarked syllable — a language with no declared prosody cannot silently get
"delete the last vowel" while claiming the stronger rule.
`rules.stress_without_prosody` says so out loud.

**Traces are deltas, not snapshots.** `Derivation { input, steps }` stores the
proto-form plus per-site edits; `replay()` reconstructs every intermediate, so
§16.3's property — the trace replays to the stored form — is a statement about
the *file*. A second `apply-rules` run extends the derivation rather than
replacing it, so a derivation always begins at the proto-form.

**The promised escalation landed:** `phonology.features_unspecified` is now an
**Error** — the M1 commitment ("this becomes an Error in M3, when a rule engine
exists") had its trigger fire. Two-sided: `generation_blocking` still filters it
so pre-M1 files keep generating, and `apply_rules` gates on the *unfiltered*
report, so the validator and the engine agree in both directions.

### Adversarial review — 6 findings, 5 fixed, 1 deferred with rationale

An adversarial panel reviewed the implementation (resolution and severity
dimensions completed; the rest were cut short by a session limit and their
findings verified by hand instead). Confirmed and fixed, now at 317 tests:

1. **`ambiguous_target_symbol` was promised and never emitted** (spec §9.5, and
   `inventory.rs` documented it as existing). When two phonemes share a bundle
   (legal — `identical_features` is only a Warning) resolution silently chose
   first-in-authored-order: exactly the Lexurgy-issue-#9 silence the design
   calls out. Now warned once per (rule, chosen phoneme) per run, naming every
   carrier; the per-site record was already in the trace's `ambiguous_with`.
2. **A convergent mint's weight depended on lexicon order.** Two different
   sources feeding one reference row kept whichever weight arrived first — word
   order reaching the evolved genome's bytes, against the module's own promise.
   The mint now keeps the **maximum** over its sources, a function of the set.
3. **`stress_without_prosody` claimed "can never fire" falsely** on lexicons
   with hand-authored stress marks, which the engine legitimately reads. The
   check now takes the lexicon and stays silent when any syllable is marked.
4. **A stale comment in `generate.rs`** still called `features_unspecified` a
   Warning — wrong in the direction that invites un-gating the engine. Fixed.
5. (Duplicate of 1, found independently by the second reviewer.)

Deferred: **`by_ipa` compares bytes, not canonical equivalence** — an author
glyph saved in NFD can evade the mint guard and leave two identically rendered
glyphs in one inventory. Closing it needs Unicode normalization data; recorded
at the comparison site and queued as an `ipa_not_nfc` warning for M7's
plausibility profile, where validation grows anyway.

### Gotchas for the next session

- **M4's fork obligation is unchanged and now demonstrated:** `evolve` copies
  every `cognate_set` verbatim (a test walks it), and `evolve` is *not* a fork —
  it produces one descendant; a fork produces sisters. The primitives are all in
  place.
- **`stemma trace` output comes entirely from `render_derivation`** in
  `stem_soundchange::view` — the CLI contributes parsing only, so the M11 UI
  calls the same function.
- **Resolution is evaluated against the input inventory,** never the growing
  one, so `Inventory` vs `Innovated` in a trace cannot flip when words reorder.
  Mints are appended in reference-table order after the run.
- **`Derivation` lives in `stem_lexicon`, not `stem_soundchange`** — the reverse
  would be a crate cycle (`stem_soundchange` already depends on `stem_lexicon`).
  The spec caught this before implementation.
- **The reference table is append-only and injective** — four construction tests
  hold it. Adding /ʃ/ would be a bug until a stridency feature exists (it would
  be byte-identical to /s/).
- **The cognate-mint source scan now walks all of `crates/*/src/`** rather than
  four hard-coded files, so `stem_soundchange` (and every future crate) is
  covered automatically.

---

## M2 — Lexicon · built 2026-07-19 · ✓ verified

Languages now have words. `stemma new-lexicon` coins one root per concept from a
built-in 103-item list, `export-md` writes a dictionary and `export-csv` a
CLDF-shaped table. **232 tests pass**; clippy clean under `-D warnings`.

Verified by running it: `new-lexicon fixtures/proto_asterian.ron --out
out/proto.ron` produces 103 entries — `aop` "all", `nuko` "ashes", `nak` "bark",
`sa` "big" — each with a stable `WordId` and `CognateSetId`; reloading yields an
identical lexicon; the dictionary and the CSV are byte-identical across runs. The
homophone check found 5 real collisions (`a`, `ni`, `nim`, `wa`) and reported them
as a Note without making the language invalid.

### Decisions worth knowing

**Export is a new crate, not a relaxation of `stem_io`.** `DESIGN.md` §9.2 puts
`markdown.rs` inside `stem_io`, but `stem_io`'s own module docs say it is generic
over serde and must not know the domain — and a Markdown dictionary cannot be
written that way, since a word's rendered form needs the phoneme inventory. The
distinction the split encodes: **persistence is total, reversible and domain-blind;
rendering is lossy, opinionated and domain-specific.** `stem_io/src/` was not
touched by this milestone, not one line. [ADR-0006](docs/adr/0006-export-is-a-separate-crate.md).

**A `concept` field that §8.3 does not list.** §8.3 offers `glosses` (free strings
that drift) and `semantic_nodes` (M9, and `SemanticNodeId` does not exist yet).
Neither can be the cross-language join key §10.3's cognate table needs, so
`ConceptKey` was added. It is deliberately **not** the cognate set: a concept is
shared *meaning*, a cognate set is shared *ancestry*, and Latin *caput* "head" →
French *chef* "chief" have one and not the other. At M2 they are in exact bijection
and a future reader will want to delete one — a test exists to stop that.
[ADR-0007](docs/adr/0007-word-identity-and-cognate-sets.md).

**Swadesh-100 does not contain `king` or `mother`** — and ROADMAP M5's own
acceptance command is `stemma cognates --meanings water sun star king mother`. The
design panel caught this. Three concepts (`MOTHER`, `KING`, `STORM`) are appended
after the hundred, under an explicit rule: *a meaning named by a Phase-1 acceptance
test or a DESIGN worked example must be representable*. Appending rather than
interleaving is what preserves the draw contract's prefix property. Without them,
M5 could not run its own test, and reopening the concept schema at M5 is exactly
the deferred cost this milestone exists to avoid.

**Concepticon anchors were fetched, not remembered.** Each Swadesh concept carries
its Concepticon id. Two values that circulate in secondary sources are wrong and
the fetched ones are used: `hair` is 1036 (not 1040) and `root` is 668 (not 670).
`KING` and `STORM` could not be verified, so their `concepticon_id` is `None` — a
plausible-looking integer under a column bearing an external authority's name would
be a false provenance claim in a program whose premise is provenance.

**`phonemic_form` is a `Root`, never a `String`.** A rendered form stored beside
the segments is a second source of truth that desynchronises the first time M3
mutates a segment, undetectably. `written()` and `ipa()` are views.

**The lexicon draws on its own RNG stream.** `RngDomain::Lexicon`, so the Nth word
of `new-lexicon --seed 42` is deliberately *not* the Nth root of `generate-roots
--seed 42`. Sharing would freeze the lexicon builder's draw budget to the root
generator's forever. `generate-roots` is a scratchpad; `new-lexicon` is an artifact.

### Gotchas for the next session

- **`build_proto_lexicon` must not call `inventory.validate()`.** `RootGenerator::new`
  deliberately filters feature-only codes out of its gate so a half-featured file
  still generates; re-validating would make `new-lexicon` refuse a language
  `generate-roots` accepts — M1's validator/engine defect reopened one axis over.
  `a_language_that_can_generate_roots_can_seed_a_lexicon` guards it.
- **Edit distance is Damerau-Levenshtein, not plain Levenshtein, and that was a
  bug fix.** Under plain Levenshtein a transposition costs 2, so `NOES` tied with
  `NOSE`, `NEW` and `NOT` — and the deterministic tie-break returned `NEW`. Since
  transposition is the most common typing error, counting it as one edit makes the
  right word win. The suggester moved to `stem_core::suggest` so features and
  concepts share one implementation.
- **`--out` rewrites the stored seed.** `--seed 7 --out f.ron` must not write a file
  saying `seed: 42` while holding words drawn from stream 7; the genome's seed
  promises reproducibility *from the file alone*, and this is the first command
  that persists a stochastic result.
- **Two golden tiers again.** The data-free canary in `stem_export` (a hand-built
  4-phoneme genome) cannot be moved by any fixture edit; the fixture goldens and
  the `d16ba861…` lexicon digest legitimately move when fixture content changes.
  Keep them distinguishable.
- **The reference fixture is unchanged and stays lexicon-less.** It is a proto
  *definition*, the thing `new-lexicon` is run against. `skip_serializing_if` keeps
  `convert` from adding an empty `lexicon: []`, so its round trip is still
  byte-identical — a test asserts it.
- **M4's whole cognate obligation is one line:** copy `cognate_set` verbatim, never
  mint. `scoped_cognate_set` is the only minting site in the workspace and a
  source-scanning test enforces it.

---

## M1 — Feature bundles & root generation · built 2026-07-19 · ✓ verified

Phonemes now carry real distinctive features, languages declare their syllable
shapes, and `stemma generate-roots` produces reproducible root words.
**157 tests pass**; clippy is clean under `-D warnings`.

Verified by running it: `generate-roots fixtures/proto_asterian.ron --count 100`
emits roots like `kanmol`, `tatsi`, `masu`, `liwa`, `sitna`. Running it twice with
the same seed is byte-identical; a different seed differs; `--count 20` is an exact
byte-prefix of `--count 100`. `stemma features` prints each segment's resolved
matrix, and a typo in a fixture (`+voicee`) fails to load with
`19:69-19:70: unknown feature '+voicee'; did you mean 'voice'?`.

### How this was designed

M1 was specified before it was written, because the feature model is what M3's
rule engine matches against and getting its shape wrong is expensive to undo. Two
rounds of adversarial design review ran first. **The first round failed usefully:**
four proposals all tried to solve M3 inside M1 — shipping feature registries, node
geometry, ordinal scales, fixpoint resolution and hand-written parsers — and
independent judges scored every one of them 5–6/10, with one verdict reading "M1
scope is over budget against the project's own top-named risk" (§20.1). The second
round was scoped explicitly to ROADMAP M1 and scored 8/7/7.

That is the origin of the single most important decision here: **M1 is much smaller
than it first appears it should be.** No rules, no classes, no scales, no
inheritance, no DSL.

### Decisions worth knowing

**The feature set is a closed enum, not a data-declared registry.** This is
counter-intuitive — §7.2's tone genesis and §7.7's alien channels both argue for an
open namespace — and it was chosen anyway because an open registry cannot tell a
new feature from a misspelled one. `+voicee` silently becomes a real feature that
no segment carries, so every rule keyed on it matches nothing forever, with no
diagnostic. Closed, that same edit is a load error with a suggestion. Safe because
**nothing on disk refers to a feature by number** — bundles serialise as signed
names, so appending a variant cannot change what any saved file means.
[ADR-0004](docs/adr/0004-closed-feature-set.md).

**Storage is ternary; reference is binary.** A cell is `+`, `−`, or absent, where
absent means "the question does not arise" (a plain alveolar has no rounding
value). Conflating absent with minus is the single most damaging error available
here: store /f/ as `[−strident]` and the class `[−strident]` silently swells from
the dental fricatives to every non-sibilant in the language. Rules may reference
only `+` and `−`, because "the segments where this is undefined" is not an attested
natural class.

**`SegmentKind` is a phonotactic slot class, not the feature `[consonantal]`.**
This is what finally resolved the glide problem cleanly. /w/ and /j/ fill consonant
slots *and* are `[-consonantal]`; both are true, and a model that infers one from
the other has to reject an author who wrote the truth. Keeping them separate also
avoided a workspace-wide refactor of `Validate`'s signature.

**ChaCha20 + SHA-256 seed expansion, with `=` version pins.** `StdRng` is
documented as non-portable and, as of rand 0.10, may change output in a *patch*
release; `seed_from_u64` reserves the same right; and rand 0.9.0's changelog says
outright that it broke `Uniform`'s value stability — which `WeightedIndex` samples
internally. Any of the three would silently rewrite every stored language on an
unrelated `cargo update`. `default-features = false` removes the non-portable
generators from the API entirely, which is the real enforcement.
[ADR-0005](docs/adr/0005-rng-and-determinism.md).

**Weights are `u32`, not `f32`** — the one deliberate back-compat break of Phase 1.
The sampler builds a prefix-sum array in iterator order; with floats, identical
weight sets draw differently depending on summation order, and quantisation can
silently zero a small weight, leaving a phoneme in the inventory and absent from
every word. Taken now because the only affected files were fixtures this milestone
rewrote anyway. It would not be worth taking at M5.

### Evidence the model will survive M3

`crates/stem_cli/tests/reference_phonology.rs` asserts, against the real fixture,
that the design doc's own example rules are expressible today:

- `[-sonorant, -continuant, -voice]` selects exactly `{p, t, k}` (§11.1 IntervocalicVoicing)
- `[+syllabic]` selects exactly `{a, e, i, o, u}` (its `V _ V` environment)
- `[+nasal]` selects exactly `{m, n}` (nasal place assimilation)
- `[-sonorant, +dorsal]` selects `{k}` and `[+syllabic, -back]` selects `{e, i}` (§11.1 VelarPalatalization)
- /i/~/j/ and /u/~/w/ differ in exactly one feature, `syllabic`

That last one is the payoff: glide formation is a single feature flip in M3 rather
than an invention of place features out of nowhere.

### What the post-implementation review caught

M1 was reviewed by five adversarial reviewers on separate dimensions, each finding
independently reproduced by a second agent before it counted. **Three reviewers
independently found the same defect**, which is the one worth remembering:

**A vowel-only language passed `validate` with a warning and then failed to
generate.** `phonology.no_consonants` is a Warning by design — CLAUDE.md says
unusual designs are flagged, not rejected — but `RootGenerator::new` built the
consonant distribution unconditionally, so `WeightedIndex::new(vec![])` errored and
leaked a raw `rand` string. The validator and the engine disagreed about whether a
language worked, which is exactly the failure the design means to make impossible.
Fixed by preparing a slot class only if some template actually uses it.

**A one-character field typo silently changed the language.** `frequency_wieght`
was accepted, the real field took its default of 10, and on the fixture's
most-frequent vowel that rewrote **17 of 20 generated roots with no diagnostic at
any severity** — a direct hit on §9.4. `Phoneme` and `LanguageGenome` now carry
`deny_unknown_fields`. This reverses a spec decision: the spec excluded them for
forward compatibility, but the contract that actually matters ("old files keep
loading") is provided by `#[serde(default)]`, not by tolerating unknown fields.

Also fixed: generation no longer blocks on feature faults it never reads (adding
features one phoneme at a time used to break a working file halfway through);
`--count 18446744073709551615` is an argument error rather than a capacity-overflow
panic; phonotactics weight sums are overflow-checked like the inventory's;
`[+round -labial]` is flagged (a /kʷ/ authored that way would escape every
`[+labial]` rule); `Root`/`Syllable` gained `Ord`/`Hash` so M2 can dedupe without a
breaking change; the two CI determinism guards no longer **fail open** (`!` inverts
only the last command's status, so a `cargo tree` error reported success — and the
reviewer verified no frozen canary would have caught the `unbiased` feature that
guard exists for); and the nucleus property test runs 1,000 seeds rather than one.

**The golden corpus digest is unchanged across all of it** — `677f3413…` before and
after — which is the evidence that none of these fixes perturbed the draw order.

### Gotchas for the next session

- **Two bugs were caught by tests written from the spec, not by review.** (1) The
  `identical_features` check sorted its collision list but not *within* each pair,
  so reversing the inventory produced `(b, a)` instead of `(a, b)` — order-dependent
  output, the exact class of defect §9.4 forbids. (2) Nothing else; the rest
  compiled and passed first time.
- **`rand::rngs::ChaCha20Rng` exists only with `features = ["chacha"]`.** Without
  it the path is `rand_chacha::ChaCha20Rng`. Both were verified by compiling.
- **rand 0.10 renamed its traits:** `Rng` is now the core trait (`next_u64`,
  `fill_bytes`) and `RngExt` carries `.random()`. In 0.9 these were `RngCore` and
  `Rng`. Remembered code from 0.8/0.9 will not compile.
- **Three data-free canaries** (seed expansion, raw keystream, weighted-index draw
  sequence) sit alongside the corpus golden digest. No fixture edit can move the
  canaries, so a red canary means the *generator* changed while a moved digest just
  means fixture content changed. Keep that distinction — it is what stops
  re-baselining the digest from becoming a reflex.
- **`features_unspecified` is a Warning, scheduled to become an Error in M3.** It
  is a Warning now only because pre-M1 files must keep loading and M1's generator
  does not read features at all. A featureless phoneme is invisible to every rule
  M3 will write.
- **Deviation on record:** templates are concrete (`CV`, `CVC`, `V`, `VC`) rather
  than the `(C)V(C)` sugar `DESIGN.md` §15 Ticket 4 names. Parenthesis expansion
  needs an optional-slot rate nothing specifies, and its expansion order would
  silently join the RNG draw order. Rejected templates point the author at the
  explicit form. Sugar can land in M2 as a load-time rewrite.

---

## M0 — Skeleton & it runs · built 2026-07-19 · ✓ verified

Stood up the Rust workspace from the design doc: seven crates, a `stemma` CLI with
`validate` / `info` / `convert`, RON+JSON project I/O, and two reference fixtures.
**51 tests pass**; `cargo clippy --workspace --all-targets -- -D warnings` is
clean; `cargo fmt` applied.

**Verified by actually running it**, not just by the test suite: `stemma validate
fixtures/proto_asterian.ron` prints `Proto-Asterian (proto_asterian) — 15 phonemes
(10C/5V), proto, seed 42` and `✓ no issues`, exit 0. The same command against
`fixtures/invalid_no_vowels.ron` reports all three planted faults
(`phonology.bad_weight`, `phonology.duplicate_ipa`, `phonology.no_nucleus`) plus a
note, and exits 1. A RON → JSON → RON round trip through `stemma convert` returns
an identical genome.

### Decisions worth knowing

**Validation returns a graded report, not a bool.** `ValidationReport` collects
`Issue`s at Error / Warning / Note. This was chosen deliberately at M0 because
`DESIGN.md` §17 wants a *plausibility profile* — "80 consonants and 2 vowels is
possible but typologically unusual" — and a boolean validator would have forced the
engine to either reject legitimate speculative designs or say nothing useful about
them. §17 is now an extension of this report rather than a separate subsystem.
Validation also never stops at the first fault; the broken fixture proves it.

**An extra crate, `stem_genome`, that isn't in the design sketch.** §9.2 puts
`language.rs` in `stem_core`, but the genome *owns* a phonology and (later) a
lexicon and rule history, so it must depend on those crates — while they depend on
`stem_core` for IDs. That is a dependency cycle. Splitting the aggregate out keeps
`stem_core` a true foundation with no internal dependencies. Recorded in
[ADR-0002](docs/adr/0002-crate-layering.md).

**IDs are readable strings, not UUIDs.** `WordId("w_0001")`, minted from a counter,
never randomly. Traceability is the product (§3.3) and a trace full of UUIDs is
unreadable; determinism (§9.4) rules out random generation anyway. Recorded in
[ADR-0003](docs/adr/0003-readable-deterministic-ids.md).

**Two crates ship empty on purpose.** `stem_lexicon` and `stem_soundchange` have
no code, only module docs stating what belongs in them and which invariants to
preserve. Fixing the dependency edges now is cheap; rearranging them later, once
types have leaked across boundaries, is not.

### Gotchas for the next session

- **Edition 2024 match ergonomics are stricter.** `.filter(|(_, &count)| …)` over a
  `HashMap` iterator is now a hard error; it needs `.filter(|&(_, &count)| …)`.
  Cost one build cycle here.
- **RON needs `implicit_some` to read hand-authored files.** Without it, an authored
  fixture must write `romanization: Some("y")`, which defeats the point of choosing
  RON for human editing. `stem_io` enables it as a *default extension when reading*
  (so fixtures parse with or without the header) but serialises through plain
  `ron::ser::to_string_pretty` — because ron only emits the `#![enable(...)]` header
  for extensions that are *not* already defaults of the `Options` doing the
  serialising. Routing writes through the configured `Options` silently produces
  files only Stemma can read. A test now pins both halves of this.
- **`HashMap` iteration order leaks into validation output.** Duplicate detection
  sorts before reporting. Watch for this everywhere output is generated —
  determinism is a hard constraint, and this is the easiest way to break it by
  accident.
- **M1 is the load-bearing one.** The feature-bundle model has to serve M3's rule
  matching. Design it whole, against §7.1 and §11, before writing code. A fixed
  struct of `Option<bool>` fields will not survive contact with tone, phonation, or
  the alien channels of §7.7 — model it as a sparse typed bundle.
