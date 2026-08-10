# 12. The rule DSL is a front end over the M3 structs, never a second engine

- **Status:** Accepted
- **Date:** 2026-08-09

## Context

M10 adds `DESIGN.md` §11.1's readable rule syntax. §20.4 names the risk it exists to
manage — "a powerful sound-change DSL can become hard to learn" — and prescribes the
order: *start with struct-based rules; add a readable DSL later*. M3 built the
structs; this milestone builds the parser, three milestones later, deliberately.

That ordering is the whole safeguard. A DSL designed *first* accumulates syntax for
things the engine cannot yet do, and the engine then grows to match — which is how a
notation becomes a second specification of the language's behaviour. Designing it
against semantics that already work inverts the pressure: the syntax can only
describe what the structs already mean.

The risk at this point is subtler than "the parser has bugs". It is that the parser
acquires *its own* semantics — a convenience that desugars slightly differently, a
default the structs do not have, a shorthand that is not quite a feature bundle —
and the project quietly acquires two definitions of what a sound change is.

## Decision

**Every path in `stem_soundchange::dsl` ends in a `SoundChangeRule`, and the
milestone's acceptance is that this is checkable rather than merely intended:**
`fixtures/rules_asterian.sc` expresses the four M3 rules in §11.1's syntax, and
applying it produces a file `cmp` reports **byte-identical** to applying the
hand-built `.ron` structs.

Two tests, deliberately separate:

- `the_dsl_parses_to_exactly_the_hand_built_structs` — the parsed `RuleSet` equals
  the deserialised one, field for field.
- `the_dsl_and_the_ron_set_produce_byte_identical_output` — the two evolved
  languages serialise to identical bytes.

They are not redundant. If the first passes and the second fails, the *engine* is
nondeterministic; if the first fails, the *parser* is wrong. One combined test would
report the same red for two unrelated defects.

### The parser has no feature table of its own

Feature names go straight to `stem_phonology`'s existing
`FeatureBundle::try_from(Vec<String>)`. There is no spelling of a feature the DSL
accepts and the engine does not, because there is only one list — and the
"did you mean `syllabic`?" suggestion on a typo comes free from the same place.

### Three deviations from §11.1's sketch, each forced

1. **A rule carries an id *and* a name.** §11.1's `rule IntervocalicVoicing:` has one
   token doing both jobs. Traces reference a stable `RuleId` while derivations print
   a human name, and `docs/adr/0003` requires ids be stable rather than derived from
   a display string an author may reword. Hence `rule r_0001 "Intervocalic voicing":`.
2. **`change: set [+voice]`, not §11.1's `voice = true`.** The bracket already means
   "a bundle of valued cells" in `target:`. Reusing it makes a multi-cell change fall
   out of the notation; `voice = true` would need a second comma syntax meaning the
   same thing.
3. **`copy` exists at all.** §11.1 has no syntax for it — every example there sets a
   literal, which `rule.rs` records as the requirement a model designed from §11.1
   alone will miss. §7.2's "nasals assimilate to the place of a following stop" is
   not `place = labial`; it is *whatever the next segment's place is*. Without
   `copy`, the DSL could not express one of the four reference rules, and the
   byte-identical claim would be unreachable.

### `V` and `C` are bundle aliases, not letter classes

§11.1 writes `environment: V _ V`, so the shorthand ships — expanding to
`[+syllabic]` and `[-syllabic]`. `v_is_exactly_the_syllabic_natural_class` pins the
equality. This does not weaken §7.1's features-not-letters rule: `V` is not
"a, e, i, o, u", it is a natural class, which is exactly why a segment a *later rule
invents* falls into it automatically. A letter class would not.

### The comment marker is `//`, not `#`

§11.1 gives `#` to the word boundary. A line-leading `# a note` is therefore
indistinguishable from a standalone boundary symbol by any *local* rule — both are a
`#` surrounded by whitespace. Every heuristic that separates them ("does the rest of
the line parse as environment items?") makes the lexer depend on the parser, which is
how a comment starts silently changing a rule. `//` collides with nothing.

### Extension dispatch lives in `stem_cli`

`load_rule_set` picks the reader by extension: `.sc` goes to the parser, anything
else to serde. It is in the CLI because `stem_io` is generic over serde and must
never learn a domain type (`docs/adr/0002`), and because `stem_soundchange` sits
below `stem_io` in the graph and does no file I/O. This is plumbing, not logic — the
parser itself is a library function, callable from the M11 UI unchanged.

## Consequences

- **`apply-rules`, `fork --rules` and `rules` accept `.sc` wherever they accepted
  `.ron`**, with no downstream change: the same `RuleSet` reaches the same unchanged
  `apply_rules`.
- **The engine is untouched.** No file under `stem_soundchange` outside `dsl.rs`
  changed except `view.rs`, and that only to render a refusal reason as prose.
- **Parse errors name the file, the line, and the offending text**, with a spelling
  suggestion — §20.4's "provide examples and templates" half. A failure the user
  cannot locate is a failure they cannot fix.
- **§20.4's "why did this rule not apply?" diagnostics were already built at M3** and
  operate at three levels: pre-flight (`target_matches_nothing`,
  `environment_matches_nothing`, `stress_without_prosody`), per run
  (`rule_never_applied`), and per word (`— did not apply`, plus refused sites with
  reasons). M10's contribution is to render those refusal reasons in **prose**
  instead of `{:?}`: a reader was previously shown
  `Unnameable { bundle: "+syllabic -voice" }`, which is the shape of the enum rather
  than the answer to their question.
- **The `.ron` format remains canonical for storage.** The DSL is an *input* format;
  nothing serialises to it, and `applied_rules` on a genome is still structs. A rule
  set is authored in whichever the user prefers and means the same thing either way.
- **Deferred, and named so scope cannot creep:** no rule *scopes* (§11.3's "nouns
  only", "coastal dialect only" — those need fields the struct does not have); no
  probabilistic rules (`probability_permille` is still unshipped); no exception
  patterns; no include/import between rule files; no formatter or `.ron` → `.sc`
  emitter; no editor tooling.
