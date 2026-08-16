// Case erosion — two ordinary sound changes, in §11.1's readable syntax (M10).
//
// NOTHING HERE KNOWS WHAT A CASE IS. These are the same kind of rules that have been
// deleting final segments since M3: they name features and a word boundary, and they
// apply to every word of the language whether or not it carries an affix. The fact
// that they happen to destroy Old Asterian's entire case system is a consequence, and
// M19 is about that consequence being *found* rather than declared.
//
// ORDER IS CHRONOLOGY (§11.3), and here it is load-bearing. `-ir` ends in a rhotic,
// so vowel loss alone would leave it as `-ir`; the rhotic has to go first, and then
// the stranded `-i` is a word-final unstressed vowel like any other. Swap the two
// rules and the ergative survives.
rules rules_case_erosion "Case erosion":
  note: "Word-final weakening, of the kind that has happened to half of Europe."

rule r_e01 "Final rhotic loss":
  note: "A word-final trill is lost."
  at: 520
  target: [+trill]
  environment: _ #
  change: delete

rule r_e02 "Final unstressed vowel loss":
  note: "A word-final vowel is lost unless its syllable bears stress."
  at: 600
  target: [+syllabic, unstressed]
  environment: _ #
  change: delete
