// The Early Asterian sound changes — §11.1's readable syntax (M10).
//
// THIS FILE IS THE ACCEPTANCE. It expresses `rules_asterian.ron` rule for rule, and
// `the_dsl_and_the_ron_set_produce_byte_identical_output` asserts that applying it
// to the reference fixture gives output identical to applying the hand-built
// structs — not merely equivalent-looking, identical. That is what makes the parser
// a FRONT END rather than a second engine (§20.4): if this file could express
// anything the structs could not, or could express the same thing differently, the
// claim would be false.
//
// ORDER IS CHRONOLOGY (§11.3), exactly as the `Vec` order is in the .ron file. The
// feeding and bleeding relationships the .ron file's header explains are properties
// of that order, so they are reproduced here by writing the rules in the same
// sequence — not by anything this syntax does.
//
// Rules name FEATURES, never letters (§7.1). `[-sonorant, -continuant, -voice]` is
// exactly {p, t, k} in this inventory, and stays correct for a segment a later rule
// invents.
rules rules_asterian_early "Early Asterian sound changes":
  note: "The chain of DESIGN.md §10.2, plus nasal place assimilation."

// §11.1's own IntervocalicVoicing example, and §10.2's rule 1.
rule r_0001 "Intervocalic voicing":
  note: "Voiceless stops voice between vowels."
  at: 250
  target: [-sonorant, -continuant, -voice]
  environment: [+syllabic] _ [+syllabic]
  change: set [+voice]

// §7.2's assimilation. `copy`, NOT `set` — the place is whatever the following
// stop's place is, and the rule does not know which. §11.1 has no syntax for this;
// see the module docs on why one had to be invented.
rule r_0002 "Nasal place assimilation":
  note: "A nasal takes the place of articulation of a following stop."
  at: 300
  target: [+nasal]
  environment: _ [-sonorant, -continuant]
  change: copy place from after[0]

// §11.1's FinalVowelLoss, and §10.2's rule 2. §11.1 writes `[+vowel, -stress]`;
// `vowel` is not a feature here (it is `+syllabic`) and stress is syllable-scoped,
// so the two halves of that bracket land on the target's two axes — which is
// exactly what this syntax does with `unstressed`.
rule r_0003 "Final unstressed vowel loss":
  note: "A word-final vowel is lost unless its syllable bears stress."
  at: 410
  target: [+syllabic, unstressed]
  environment: _ #
  change: delete

// §10.2's rule 3. Its target class is EMPTY in Proto-Asterian — the only segment it
// can ever apply to is one r_0001 created. That is feeding order, and it is why the
// engine has no inventory-membership fast path in matching.
rule r_0004 "Velar lenition":
  note: "The voiced velar stop spirantises between vowels."
  at: 480
  target: [-sonorant, -continuant, +voice, +dorsal]
  environment: [+syllabic] _ [+syllabic]
  change: set [+continuant]
