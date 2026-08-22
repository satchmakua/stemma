// A SIGNAL CHANGE — the M24 acceptance fixture, and the whole argument of the milestone.
//
// These are sound changes. They are written in §11.1's syntax, parsed by M10's parser,
// and applied by `apply_rules` — the same function that turned `*takala` into `taɣal`
// at M3. NOTHING IN THE ENGINE WAS CHANGED to let them run over pulses of light.
//
// That is the claim M24 exists to make, and the reason it is credible is that the
// engine never asked what produced a segment. It matches feature bundles, rewrites
// feature bundles, and resolves the result back to a symbol. A vocal tract lives
// entirely in *which dimensions a unit values* — and these rules name `[+luminous]`
// dimensions where an Asterian rule names `[+syllabic]` ones.
//
// ORDER IS CHRONOLOGY (§11.3), and here it is LOAD-BEARING in the M19 way. r_k01 makes
// every slow flicker rapid; r_k02 then catches every rapid flicker between two steady
// glows. A word whose slow flicker sits between two glows is dimmed by the pair —
// and only in this order. Swap them and r_k02 runs first, finds that flicker still
// slow, passes over it, and the word comes out unchanged.
//
// A NOTE ON WHAT THESE CANNOT DO. Neither rule innovates a signal. M3's resolver mints
// a new phoneme from a compiled-in REFERENCE TABLE of reviewed IPA rows — and there is
// no such table for light, because there is no attested inventory of bioluminescent
// signals to review. Inventing one would be fabricating a canon for a channel nobody
// has ever studied, which is the kind of claim this project refuses to make.
//
// So a Kethi signal change is a MERGER or nothing: the output bundle must already be a
// signal the author declared. A rule producing an undeclared bundle earns
// `unnameable_output` and leaves the site alone — the engine's ordinary behaviour,
// doing the right thing here for a reason nobody had to add.
rules rules_kethi_dimming "The rapid merger and interglow dimming":
  note: "Two changes in the mantle bands, four hundred years apart."

// The slow flicker speeds up and merges with the quick one. Both are already
// [+bright +long_wave +saturated +pulsed]; the pulse RATE was the only thing between
// them, and now it is not. A merger: two signals become one, and every word that told
// them apart stops doing so.
rule r_k01 "Rapid merger":
  note: "A slow flicker becomes a rapid one, merging the two pulse rates."
  at: 300
  target: [+luminous, +pulsed, -rapid]
  change: set [+rapid]

// And a rapid flicker dims between two steady glows — the luminous analogue of
// intervocalic voicing, and deliberately so: the environment is a feature bundle on
// either side, and the engine's adjacency window does not care whether the neighbours
// are vowels or light.
rule r_k02 "Interglow dimming":
  note: "A bright rapid flicker dims and cools between two steady glows."
  at: 700
  target: [+luminous, +pulsed, +rapid, +bright]
  environment: [+luminous, -pulsed] _ [+luminous, -pulsed]
  change: set [-bright, -long_wave]
