// Two ordinary sound changes, in §11.1's readable syntax (M10) — and the M21 fixture.
//
// NEITHER RULE KNOWS THERE IS A SCRIPT. These are the same kind of changes that have
// been moving segments around since M3: they name feature bundles and an environment,
// and they would run identically on a language nobody had ever written down. That is
// the point. §7.6's claim is that a glyph's history and a word's history are
// **independent**, and the only honest way to show it is to move the sounds with a
// process that has never heard of the spelling.
//
// What they do to Written Asterian's three scripts is a consequence, found afterwards
// by `stemma scripts` and `stemma glyph-trace`:
//
//   r_g01 GLIDE LOSS deletes /w/ and /j/ outright. Every sign for them — `g_w`, `g_y`
//   in the alphabet, `gt_w`, `gt_y` in the abjad — is left writing a sound no word of
//   the language contains any more. Those signs have OUTLIVED THEIR SOUND, which is
//   §7.6's claim in one line, and the engine finds it by reading the word list.
//
//   r_g02 INTERVOCALIC VOICING mints /b/, /d/, /ɡ/, which the Kirran alphabet has no
//   letters for — it was cut for the fifteen sounds Asterian had when the clerks
//   invented it, and nobody has cut new punches since. The SPELLING CANNOT KEEP UP.
//
// Together those are the two directions an orthography and a language come apart, and
// they are the two halves `ScriptDrift` measures. English has both: the `gh` of
// *night* is a fossil, and there is no letter for the sound in the middle of *measure*.
//
// ORDER IS CHRONOLOGY (§11.3). Glide loss runs first, so a /w/ between two vowels is
// gone before voicing could have looked at it — and voicing is about stops anyway, so
// unlike M19's case erosion the order here is not load-bearing. It is stated because
// §11.3 says order is always a claim, even when nothing turns on it.
rules rules_glide_loss "Glide loss and intervocalic voicing":
  note: "Two changes that between them leave the script four hundred years behind."

rule r_g01 "Glide loss":
  note: "The glides are lost everywhere — a sound change with no interest in the fact that four signs were made for them."
  at: 300
  target: [-consonantal, -syllabic]
  change: delete

rule r_g02 "Intervocalic voicing":
  note: "A voiceless stop voices between two vowels. It innovates three sounds the alphabet has no letters for."
  at: 450
  target: [-sonorant, -continuant, -voice]
  environment: [+syllabic] _ [+syllabic]
  change: set [+voice]
