# Stemma: A Scientific Language-Evolution Workbench

**Project type:** research-grade creative software, computational linguistics engine, conlang workbench, xenolinguistics sandbox  
**Primary implementation target:** Rust core engine + TypeScript/Tauri or web UI  
**Document version:** 0.2 (renamed from the working title *Exoglossia*)  
**Name origin:** in textual criticism, a *stemma* (pl. *stemmata*) is the reconstructed family tree showing how surviving manuscript copies descend from a lost original — attested descendants, branch points, inferred ancestors. That is this program's core data structure. The crate prefix `stem_` doubles as the morphological sense of *stem*.  
**Primary goal:** build a rigorous program for generating, evolving, forking, inspecting, parsing, translating, and writing plausible fictional languages, including deeply alien languages whose structures are shaped by non-human embodiment and communication channels.

---

> ## How this document is used
>
> **This is the single source of truth.** When the design and the code disagree,
> the design wins — or this doc is wrong and should be fixed first. Keep it
> current: a stale spec is worse than none, because it will still be followed.
>
> It has three companions:
>
> | Doc | Role |
> |---|---|
> | [ROADMAP.md](ROADMAP.md) | Turns §13–§14 into the working build order, with explicit **Test** steps per milestone. **This is the checklist to build from.** |
> | [PROGRESS.md](PROGRESS.md) | The backward-looking log: what shipped, and how it was verified. |
> | [CLAUDE.md](CLAUDE.md) | Standing build instructions — how to run and test, the hard constraints, the architecture map. |
>
> Where this document and `ROADMAP.md` differ on sequencing, **ROADMAP wins** —
> it is the live plan and records deviations as they are decided. Where they
> differ on *intent*, this document wins.
>
> Section references like "§7.1" are used throughout the code comments to point
> back here; keep the numbering stable.

---

## 1. Executive Summary

Stemma is a language-evolution laboratory for constructed languages. It is not merely a random word generator, a dictionary app, or a simple sound-change applier. It treats an artificial language as a structured, evolving organism: a system with phonology, morphology, syntax, semantics, lexicon, pragmatics, orthography, script history, sociolinguistic context, and lineage.

The central user experience is this:

1. Create or generate a proto-language.
2. Define its phonology, word shapes, morphology, syntax, semantics, and writing system.
3. Apply historically plausible changes over time.
4. Fork the language into daughter languages, dialects, registers, creoles, liturgical forms, or alien-contact descendants.
5. Inspect every word, glyph, grammar feature, and irregularity through its causal history.
6. Parse and generate sentences through a structured semantic representation.
7. Export grammars, dictionaries, cognate tables, inscriptions, interlinear glosses, and language-family trees.

The core principle is:

> Every generated weirdness must have a traceable causal history.

A word should not simply look strange. It should look strange because unstressed vowels deleted, consonant clusters simplified, a suffix fused to the stem, the word was borrowed from a prestige language, or a taboo replacement displaced the original form. A script character should not merely be decorative. It should descend from a pictogram, rebus form, syllabic sign, ligature, manuscript abbreviation, featural diagram, or alien perceptual convention.

The long-term ambition is to support both human-plausible and alien-plausible languages. Human-plausible languages are grounded in typology, historical linguistics, phonetics, grammaticalization, and semantic drift. Alien-plausible languages extend those principles into non-human communication channels: bioluminescent pulse grammars, chemical scent languages, ultrasonic braid languages, electrical-field communication, tentacular gesture grammar, magnetic navigation deixis, and distributed hive speech.

The MVP should be deliberately narrower: a diachronic lexicon engine with phoneme inventories, root generation, ordered sound changes, language forking, cognate tables, and word-trace inspection. Once that foundation works, morphology, syntax, script evolution, semantics, parsing, translation, and alien modality can be layered in systematically.

---

## 2. Problem Statement

Most conlang tools are useful but narrow. They usually help with one of these tasks:

- generating words that match phonotactic patterns;
- applying sound changes to a word list;
- storing dictionary entries;
- writing grammar notes;
- producing scripts or glyphs visually;
- managing a worldbuilding wiki.

These are valuable, but they often fail to model language as a historical system. A fictional language becomes more convincing when its irregularities, cognates, morphology, syntax, writing system, and semantic field have histories. Natural languages are full of fossilized processes: old case endings, collapsed compounds, borrowed prestige forms, sound changes that created irregular alternations, reanalyzed particles, taboo replacements, analogical leveling, scribal conventions, and dialect mixtures.

Stemma addresses this by treating a language as an evolving graph of constraints and transformations rather than a static collection of words.

The project should answer questions like:

- Given a proto-language, what are three plausible descendants after 1,500 years?
- How would final-vowel deletion affect case endings, verb agreement, and word order?
- What happens when a language with rich suffixing morphology undergoes stress shift and unstressed vowel loss?
- How would a classical script evolve into modern cursive, block print, and machine encoding forms?
- What does a dictionary look like when every entry has etymology, cognates, register, and semantic drift?
- How could a non-human species with four manipulatory limbs, color-changing skin, and ultrasonic perception develop a grammar unlike human speech but still internally lawful?

The program should make fictional languages feel discovered rather than invented.

---

## 3. Design Pillars

### 3.1 Diachrony First

The most important feature is historical evolution. Languages should change over simulated time. The system should represent proto-languages, intermediate stages, daughter languages, dialects, registers, contact languages, and modern descendants.

### 3.2 Formal Core, Creative Surface

The underlying language model should be formal, explicit, and deterministic where possible. Creative interfaces and AI assistance can sit on top, but the source of truth must be a structured linguistic engine.

### 3.3 Traceability

Every generated form should be explainable. A user should be able to click a word, morpheme, sound, construction, or glyph and ask: why is it like this? The system should provide a trace.

### 3.4 Plausibility Over Randomness

The system should not merely produce pronounceable strings. It should use typological constraints, historical processes, feature-based phonology, and semantic pathways to produce plausible results. Alien languages may be extremely divergent, but their divergence should arise from embodiment and channel constraints, not arbitrary weirdness.

### 3.5 Forkability

Languages should be version-controlled organisms. A language can branch into daughter languages, dialects, registers, creoles, standard forms, liturgical archaisms, and future descendants. Users should be able to diff, merge, fork, tag, and compare these lineages.

### 3.6 Human and Alien Modes

The tool should support two broad families of language design:

- **Anthroglossia:** human-plausible languages constrained by human phonetics, cognition, historical linguistics, and typology.
- **Xenoglossia:** alien-plausible languages constrained by non-human embodiment, sensory ecology, social structure, and communication channel.

### 3.7 Build From the Smallest Serious Kernel

The first version should not attempt full translation, full syntax, AI integration, or alien glyph rendering. It should prove the core diachronic model with phonology, sound change, lexicon evolution, language branching, and etymology tracing.

---

## 4. Reference Landscape

Stemma should learn from existing tools and datasets without simply duplicating them.

Useful reference points:

- **PHOIBLE**: a repository of cross-linguistic phonological inventory data, useful as a model for phoneme inventory representation and feature-based phonological constraints.  
  https://phoible.org/

- **WALS, the World Atlas of Language Structures**: a major typological database of structural properties across languages, useful for plausibility scoring and grammar-parameter priors.  
  https://wals.info/

- **CLDF, Cross-Linguistic Data Formats**: a specification for interoperable cross-linguistic data, useful as an inspiration for import/export and schema design.  
  https://cldf.clld.org/

- **Lexurgy**: a sound-change applier for simulating historical changes in word lists, useful as a reference for sound-change DSL ergonomics and trace output.  
  https://www.lexurgy.com/sc

- **SCA² / Sound Change Applier**: a widely used sound-change tool in the conlang community, useful as a benchmark for feature coverage and rule usability.  
  https://www.zompist.com/scahelp.html

The opportunity is to integrate these types of capabilities into a broader system where sound changes interact with morphology, semantics, scripts, and language-family history.

---

## 5. Target Users

### 5.1 Primary Users

- Conlangers who want historical depth and controllable evolution.
- Fantasy and science-fiction writers who want convincing languages without manually designing every rule.
- Game developers building alien cultures, fantasy civilizations, or procedural worlds.
- TTRPG worldbuilders who want language families, naming systems, inscriptions, and maps.
- Linguistics enthusiasts who want a playful but rigorous way to explore historical linguistics.

### 5.2 Secondary Users

- Linguistics students learning sound change, typology, cognates, and grammaticalization.
- Computational linguistics researchers prototyping synthetic language data.
- AI researchers interested in structured synthetic languages and constrained generation.
- Visual artists designing scripts, glyph systems, and non-human semiotic systems.

---

## 6. Product Modes

Stemma should support several workflows, but they should share one underlying engine.

### 6.1 Conlanger Workbench

A practical tool for building and documenting languages.

Features:

- phoneme inventory editor;
- phonotactic pattern builder;
- word/root generator;
- lexicon manager;
- sound-change timeline;
- morphology notes;
- grammar sketch generator;
- writing-system designer;
- exportable dictionary and grammar.

### 6.2 Diachronic Evolution Simulator

A scientific sandbox for modeling language change.

Features:

- proto-language generation;
- ordered sound changes;
- branching family trees;
- cognate tables;
- etymology traces;
- morphology erosion;
- analogy and irregularity;
- contact and borrowing;
- typological plausibility scoring.

### 6.3 Alien Language Laboratory

A speculative xenolinguistics module.

Features:

- non-human vocal tracts;
- non-acoustic channels;
- simultaneous multimodal grammar;
- non-linear writing systems;
- embodiment-driven categories;
- alien deixis, pronouns, evidentiality, temporality, and social marking.

### 6.4 Language-Family Story Engine

A worldbuilding-oriented simulation layer.

Features:

- migrations;
- isolation;
- conquest;
- standardization;
- script reform;
- religious archaism;
- prestige borrowing;
- trade pidgins;
- creoles;
- dialect continua;
- language death and revival.

### 6.5 LLM-Augmented Copilot

A constrained AI assistant that operates on the formal model.

Features:

- explain a sound change;
- propose plausible daughter-language changes;
- generate cultural names under constraints;
- write a grammar sketch from structured data;
- flag typological oddities;
- suggest alien embodiment implications;
- translate small phrases using the formal grammar where possible.

The LLM must not be the source of truth. It should be an assistant, tutor, explainer, and creative suggester constrained by the deterministic language model.

---

## 7. Scientific Model

Stemma should be grounded in several linguistic domains.

### 7.1 Phonetics and Phonology

The system represents sounds as structured feature bundles rather than raw letters. A phoneme has place, manner, voicing, height, backness, rounding, length, tone, phonation, and other relevant features depending on type.

Example conceptual representation:

```text
/p/ = consonant, bilabial, stop, voiceless, oral
/b/ = consonant, bilabial, stop, voiced, oral
/m/ = consonant, bilabial, nasal, voiced
/i/ = vowel, high, front, unrounded, short
/u/ = vowel, high, back, rounded, short
```

This enables natural sound-change rules:

```text
voiceless stops become voiced between vowels
nasals assimilate to the place of a following stop
front vowels trigger palatalization of preceding velars
unstressed final vowels delete
```

### 7.2 Historical Sound Change

The diachronic engine supports ordered transformations over phonological representations.

Core change types:

- lenition;
- fortition;
- assimilation;
- dissimilation;
- palatalization;
- labialization;
- intervocalic voicing;
- final devoicing;
- vowel raising/lowering;
- vowel fronting/backing;
- vowel harmony;
- syncope;
- apocope;
- epenthesis;
- metathesis;
- cluster simplification;
- tone genesis;
- phonemic merger;
- phonemic split;
- stress shift.

Each rule should be ordered and traceable.

### 7.3 Morphology

The morphology engine represents words as structured morpheme assemblies. It should support isolating, agglutinative, fusional, polysynthetic, introflexive/templatic, head-marking, dependent-marking, suffixing, prefixing, infixing, reduplicating, and classifier-heavy systems.

Morphology should evolve through grammaticalization and erosion.

Examples:

```text
free pronoun → clitic → agreement suffix
postposition → case ending
word meaning “go” → future marker
demonstrative → definite article
numeral “one” → indefinite article
noun meaning “body” → reflexive marker
```

### 7.4 Syntax

The syntax engine represents constraints and constructions rather than merely word order labels.

Initial syntactic parameters:

- basic word order;
- head directionality;
- adposition placement;
- genitive/noun order;
- adjective/noun order;
- case alignment;
- relative clause strategy;
- negation position;
- question formation;
- topic/focus marking;
- pro-drop;
- evidentiality;
- switch-reference.

Syntax should evolve. Case erosion may force stricter word order. Contact may introduce calqued constructions. Topic markers may become articles. Serial verbs may become auxiliaries.

### 7.5 Semantics

The system should store meanings as semantic nodes rather than simple English glosses. Words can map to one or more semantic nodes and can drift through metaphor, metonymy, narrowing, broadening, pejoration, amelioration, taboo replacement, religious elevation, technical specialization, and slang inversion.

Example drift:

```text
hand → control → authority
breath → life → soul
star → divine sign → royal title
eat → consume → understand
sharp → clever → cruel
mother → source → river-mouth
```

### 7.6 Writing Systems

Orthographies and scripts should have histories. A script can evolve from pictograms to logograms, through rebus usage, into syllabaries, abjads, alphabets, abugidas, featural systems, manuscript ligatures, print standards, and digital encoding conventions.

A glyph should have ancestry just like a word.

Example glyph history:

```text
six-point star pictogram
→ logogram for STAR
→ divine determinative
→ rebus sign for syllable /sa/
→ simplified three-stroke manuscript form
→ modern royal-name marker
```

### 7.7 Alien Modality

Alien languages should be derived from embodiment and channel constraints.

Possible channels:

- ultrasonic pulses;
- infrasonic resonance;
- bioluminescent color bands;
- chromatophore skin patterns;
- tentacular gesture grammar;
- electrical field pulses;
- chemical scent packets;
- pressure waves through liquid;
- magnetic orientation shifts;
- multi-speaker hive harmonics.

The system should infer or guide plausible linguistic consequences.

Examples:

- A chemical language may favor slow but persistent utterances, delayed deixis, environmental marking, and evidential traces.
- A bioluminescent species may use simultaneous grammar: color for mood, pulse rhythm for tense, spatial position for argument role.
- A hive species may encode subgroup identity obligatorily and lack ordinary singular pronouns.
- A tentacular species may use simultaneous classifiers expressed by limb position.
- A magnetoceptive species may encode direction, migration path, and home-vector in ordinary verbs.

---

## 8. Core Data Model

The central abstraction is the **Language Genome**.

### 8.1 Language Genome

```rust
pub struct LanguageGenome {
    pub id: LanguageId,
    pub name: String,
    pub parent: Option<LanguageId>,
    pub lineage_depth_years: i32,
    pub phonology: Phonology,
    pub phonotactics: Phonotactics,
    pub prosody: Prosody,
    pub morphology: MorphologyProfile,
    pub syntax: SyntaxProfile,
    pub semantics: SemanticSpace,
    pub lexicon: Lexicon,
    pub writing_systems: Vec<WritingSystem>,
    pub sociolinguistics: SociolinguisticProfile,
    pub history: Vec<HistoricalEvent>,
}
```

### 8.2 Phoneme

```rust
pub struct Phoneme {
    pub id: PhonemeId,
    pub ipa: String,
    pub romanization: Option<String>,
    pub kind: SegmentKind,
    pub features: FeatureBundle,
    pub frequency_weight: f32,
    pub markedness_score: f32,
}
```

### 8.3 Word Entry

```rust
pub struct WordEntry {
    pub id: WordId,
    pub form: SurfaceForm,
    pub phonemic_form: PhonologicalForm,
    pub glosses: Vec<Gloss>,
    pub semantic_nodes: Vec<SemanticNodeId>,
    pub part_of_speech: PartOfSpeech,
    pub morphemes: Vec<MorphemeId>,
    pub source: WordSource,
    pub ancestor: Option<WordId>,
    pub cognate_set: Option<CognateSetId>,
    pub register: Register,
    pub frequency: f32,
    pub usage_notes: Vec<String>,
    pub trace: EvolutionTrace,
}
```

### 8.4 Sound Change Rule

```rust
pub struct SoundChangeRule {
    pub id: RuleId,
    pub name: String,
    pub description: String,
    pub target: SegmentPattern,
    pub replacement: SegmentTransformation,
    pub environment: EnvironmentPattern,
    pub exceptions: Vec<ExceptionPattern>,
    pub probability: f32,
    pub chronology: Chronology,
}
```

### 8.5 Historical Event

```rust
pub enum HistoricalEventKind {
    SoundChange(SoundChangeRule),
    MorphologicalChange(MorphologicalChange),
    SemanticShift(SemanticShift),
    Borrowing(ContactEvent),
    ScriptReform(ScriptChange),
    DialectSplit(DialectSplit),
    Standardization(StandardizationEvent),
    AlienModalityShift(ModalityChange),
}

pub struct HistoricalEvent {
    pub id: EventId,
    pub date_range: DateRange,
    pub kind: HistoricalEventKind,
    pub affected_items: Vec<EntityRef>,
    pub explanation: String,
}
```

### 8.6 Lineage Graph

```rust
pub struct LanguageLineageGraph {
    pub nodes: HashMap<LanguageId, LanguageGenome>,
    pub edges: Vec<LineageEdge>,
}

pub enum LineageEdgeKind {
    Descent,
    DialectSplit,
    ContactInfluence,
    Creolization,
    Standardization,
    ScriptBorrowing,
}
```

---

## 9. Architecture

### 9.1 Recommended Technology Stack

```text
Core engine: Rust
Serialization: serde + RON/JSON
Database: SQLite via sqlx or rusqlite
Rule parsing: chumsky, pest, or nom
Frontend: Tauri + Svelte/React, or web-only TypeScript
Visualization: D3, Cytoscape, Canvas, SVG, or WebGPU later
Optional AI layer: local or API-based LLM service
Exports: Markdown, HTML, JSON, CSV, CLDF-inspired tables, LaTeX later
```

Rust is the best fit for the formal engine: transformations, graph operations, parsers, validation, simulation, and reproducible pipelines. TypeScript is the best fit for the UI: timeline views, tree views, graph exploration, script rendering, editor panels, and interactive visualization.

### 9.2 Crate Layout

```text
stemma/
  Cargo.toml
  README.md
  docs/
    design.md
    rule_dsl.md
    data_model.md
    roadmap.md
  crates/
    stem_core/
      src/
        lib.rs
        ids.rs
        errors.rs
        language.rs
        lineage.rs
    stem_phonology/
      src/
        lib.rs
        phoneme.rs
        features.rs
        inventory.rs
        phonotactics.rs
        prosody.rs
    stem_soundchange/
      src/
        lib.rs
        rule.rs
        parser.rs
        apply.rs
        trace.rs
    stem_lexicon/
      src/
        lib.rs
        word.rs
        morpheme.rs
        cognate.rs
        etymology.rs
    stem_morphology/
      src/
        lib.rs
        profile.rs
        paradigms.rs
        grammaticalization.rs
    stem_syntax/
      src/
        lib.rs
        constructions.rs
        alignment.rs
        generator.rs
    stem_semantics/
      src/
        lib.rs
        semantic_graph.rs
        drift.rs
        interlingua.rs
    stem_script/
      src/
        lib.rs
        glyph.rs
        script.rs
        evolution.rs
        renderer.rs
    stem_xeno/
      src/
        lib.rs
        modality.rs
        embodiment.rs
        channel.rs
    stem_io/
      src/
        lib.rs
        ron.rs
        json.rs
        markdown.rs
        csv.rs
    stem_cli/
      src/
        main.rs
  apps/
    desktop/
      src-tauri/
      src/
        main.ts
        App.svelte
        views/
        components/
        stores/
```

For MVP, this can be collapsed into fewer crates. However, the design should preserve clean module boundaries.

### 9.3 Engine Pipeline

Basic evolution pipeline:

```text
Load language genome
Load module/rule definitions
Validate phonology and lexicon
Apply historical event sequence
Generate descendant language
Record traces for each transformed item
Update lineage graph
Persist new language version
Render views and exports
```

### 9.4 Determinism

All stochastic generation should be seedable.

Requirements:

- every generated language has a seed;
- every evolution run stores its seed and rule sequence;
- re-running the same pipeline should reproduce the same result;
- random decisions should be traceable when they affect word forms, semantic drift, or grammar changes.

---

## 10. Core User Interface

The UI should feel like a cross between a code editor, a linguistics lab, a genealogy browser, and a worldbuilding tool.

### 10.1 Main Views

```text
Project Dashboard
Language Genome View
Phonology Inventory View
Phonotactics Builder
Sound Change Timeline
Lexicon / Dictionary View
Cognate Table View
Word Trace Inspector
Grammar Profile View
Script Evolution View
Sentence Parser / Generator View
Translation Lab
Family Tree View
Alien Modality Designer
Export Center
```

### 10.2 The Killer Feature: Trace This Word

Clicking any word should open a full trace:

```text
Modern form
Pronunciation
Gloss and semantic nodes
Part of speech
Morphological decomposition
Ancestor forms
Applied sound changes
Semantic shifts
Borrowing events
Register changes
Script forms across historical stages
Cognates in sister languages
```

Example trace:

```text
Proto: *takala “star, bright thing”
Rule 1: intervocalic /k/ > /g/ → tagala
Rule 2: unstressed final vowel loss → tagal
Rule 3: /g/ lenites to /ɣ/ between vowels → taɣal
Semantic shift: “star” → “omen” in priestly register
Script change: STAR pictogram becomes syllabic sign TA
Modern Coastal: taal “omen, royal sign”
Modern Highland: tazal “star”
```

### 10.3 Cognate Table View

A table comparing descendant forms by meaning and cognate set.

```text
Meaning     Proto       Coastal      Highland      Trade Creole
sun         *sawel      sol          shaur         sao
water       *akwa       awa          ak            wa
king        *rekan      ren          rihan         kan
star        *takala     taal         tazal         tala
```

Each cell should be clickable.

### 10.4 Sound Change Timeline

A timeline of ordered events:

```text
0 CE      Proto-Asterian
250 CE    Intervocalic voicing
410 CE    Final vowel deletion
600 CE    Palatalization before front vowels
780 CE    Case suffix erosion
900 CE    Coastal /p t k/ lenition
1100 CE   Script reform under Maritime Empire
```

Users should be able to disable, reorder, clone, or fork from a point in the timeline.

### 10.5 Script Evolution View

Show a glyph’s history visually:

```text
Pictogram → Logogram → Syllabic sign → Manuscript form → Print form → Digital glyph
```

The MVP can represent this textually. A later version can include vector glyph generation.

---

## 11. Rule DSL

A rule DSL is essential. It should be readable for conlangers but formal enough for deterministic execution.

### 11.1 MVP Rule Format

Example:

```text
rule IntervocalicVoicing:
  target: [-sonorant, -continuant, -voice]
  change: voice = true
  environment: V _ V
```

Example:

```text
rule FinalVowelLoss:
  target: [+vowel, -stress]
  change: delete
  environment: _ #
```

Example:

```text
rule VelarPalatalization:
  target: [place=velar]
  change: place = palatal
  environment: _ [vowel, front]
```

### 11.2 Trace Output

Applying rules should produce structured traces:

```json
{
  "word_id": "w_001",
  "input": "takala",
  "rule": "IntervocalicVoicing",
  "matches": [
    {
      "span": [2, 3],
      "before": "k",
      "after": "g",
      "environment": "a _ a"
    }
  ],
  "output": "tagala"
}
```

### 11.3 Rule Priorities

Rules are ordered by chronology. Later versions may include probabilistic, dialectal, lexical, morphological, or register-sensitive rules.

```text
Rule scope examples:
  all words
  nouns only
  unstressed syllables only
  high-frequency words only
  coastal dialect only
  borrowed words excluded
  ritual register only
```

---

## 12. MVP Definition

The MVP should be intentionally focused.

### 12.1 MVP Goal

Build a deterministic diachronic lexicon engine that can:

1. Create a proto-language with a phoneme inventory and phonotactics.
2. Generate a root lexicon.
3. Define ordered sound-change rules.
4. Fork the language into daughter languages.
5. Apply different rule histories to each daughter.
6. Display cognate tables.
7. Trace each word’s evolution.
8. Export the result as Markdown/CSV/JSON.

### 12.2 MVP Non-Goals

Do not include these in the first milestone:

```text
No full syntax engine
No full translation engine
No LLM integration
No procedural glyph rendering
No alien language modality engine
No complex morphology
No audio synthesis
No web publishing platform
No multiplayer/collaboration
No full CLDF compliance
No large typology database ingestion
```

### 12.3 MVP Success Demo

The first serious demo should show:

1. Proto-language with 100 generated roots.
2. Three daughter languages.
3. Different sound-change histories for each daughter.
4. A cognate table of 25 meanings.
5. A word trace for five selected words.
6. A Markdown export containing language sketch, sound changes, dictionary, cognates, and etymologies.

---

## 13. Roadmap

### Phase 0: Project Skeleton

**Goal:** create the repository, Rust workspace, CLI, and minimal data structures.

Tasks:

- initialize Rust workspace;
- create `stem_core`, `stem_phonology`, `stem_soundchange`, `stem_lexicon`, and `stem_cli`;
- define ID types and error types;
- add serde support;
- create basic test framework;
- create sample project fixture.

Success criteria:

- `cargo test` passes;
- CLI runs `stemma --help`;
- sample language file can load and validate.

### Phase 1: Phonology Kernel

**Goal:** represent phoneme inventories and phonotactic constraints.

Tasks:

- define segment kinds;
- define feature bundles;
- define phoneme inventory;
- define syllable templates;
- implement weighted random phoneme selection;
- implement root generation from phonotactics;
- validate generated roots.

Success criteria:

- system can generate 100 roots matching a simple `(C)V(C)` template;
- roots use only legal phonemes;
- generated roots are reproducible from seed;
- tests cover inventory validation and root generation.

### Phase 2: Lexicon Kernel

**Goal:** store roots, glosses, semantic placeholders, and word IDs.

Tasks:

- define `WordEntry`;
- define gloss and part-of-speech enums;
- define lexicon storage;
- implement import/export JSON/RON;
- generate starter lexicon from a Swadesh-like internal concept list;
- assign cognate-set IDs.

Success criteria:

- CLI can generate a proto-language lexicon;
- lexicon exports to Markdown and CSV;
- every word has stable ID and cognate-set ID.

### Phase 3: Sound Change Rule Engine

**Goal:** apply ordered phonological transformations to word forms.

Tasks:

- define sound-change rule data model;
- implement segment pattern matching;
- implement environment matching;
- implement replacement/delete/insert/feature-change operations;
- implement ordered rule application;
- implement rule trace records.

Success criteria:

- rules can voice intervocalic stops;
- rules can delete final unstressed vowels;
- rules can assimilate nasal place;
- every transformed word has trace output;
- tests cover simple and chained changes.

### Phase 4: Language Forking and Lineage

**Goal:** create daughter languages with independent histories.

Tasks:

- implement `LanguageLineageGraph`;
- implement fork operation;
- store parent/child relationships;
- apply rule sequences to daughter languages;
- preserve cognate IDs across descendants;
- implement lineage export.

Success criteria:

- one proto-language can fork into three daughters;
- daughters have different transformed lexicons;
- cognate table shows shared ancestry;
- traces preserve the path from proto to modern form.

### Phase 5: CLI Demo and Markdown Export

**Goal:** produce a compelling terminal-based MVP.

Tasks:

- create CLI commands:
  - `stemma new`;
  - `stemma generate-roots`;
  - `stemma apply-rules`;
  - `stemma fork`;
  - `stemma cognates`;
  - `stemma trace-word`;
  - `stemma export-md`;
- implement Markdown export;
- implement CSV export;
- create demo project.

Success criteria:

- running one scripted demo creates a proto-language, three daughters, cognate tables, and Markdown output;
- the exported file is readable as a language-family sketch.

### Phase 6: Desktop/Web UI Prototype

**Goal:** build the first visual interface.

Tasks:

- choose Tauri + Svelte/React or web-only architecture;
- create project dashboard;
- create phoneme inventory view;
- create lexicon table;
- create sound-change timeline;
- create cognate table;
- create word-trace inspector;
- connect UI to Rust backend.

Success criteria:

- user can open a project visually;
- user can inspect words and traces;
- user can view daughter languages side-by-side.

### Phase 7: Morphology v0

**Goal:** add basic morpheme-aware words.

Tasks:

- define morpheme entries;
- represent stems, affixes, clitics, and compounds;
- support simple suffixing/prefixing morphology;
- apply sound changes across morpheme boundaries;
- track fossilized forms;
- implement simple paradigm tables.

Success criteria:

- language can generate simple noun/verb paradigms;
- sound changes can create irregular paradigms;
- traces show morpheme-level history.

### Phase 8: Semantics v0

**Goal:** move beyond English gloss strings.

Tasks:

- define semantic graph nodes;
- link words to semantic nodes;
- implement semantic drift events;
- implement taboo replacement;
- implement metaphor/metonymy templates;
- display semantic history in word trace.

Success criteria:

- word meanings can shift over time;
- dictionary entries show semantic ancestry;
- cognate words can diverge semantically.

### Phase 9: Script System v0

**Goal:** add orthography and script history.

Tasks:

- define grapheme inventory;
- define romanization rules;
- define phoneme-to-grapheme mapping;
- define glyph ancestry records;
- support script reform events;
- export sample inscriptions.

Success criteria:

- a language can have an orthography distinct from phonemic form;
- script reforms alter spellings over time;
- word trace includes written forms across stages.

### Phase 10: Syntax and Interlingua v0

**Goal:** generate and parse simple sentences.

Tasks:

- define semantic proposition format;
- define simple noun phrase and verb phrase structures;
- implement word-order parameters;
- implement case/agreement placeholders;
- generate simple sentences from semantic input;
- produce interlinear gloss output.

Success criteria:

- system can render a semantic proposition into multiple daughter languages;
- output includes literal gloss and free translation;
- syntax differences are visible across branches.

### Phase 11: Alien Modality Prototype

**Goal:** represent non-human communication channels.

Tasks:

- define modality model;
- define channel capacity and simultaneity parameters;
- define alien feature categories;
- create bioluminescent pulse language prototype;
- create chemical scent language prototype;
- create gesture/tentacle language prototype;
- render non-linear utterance diagrams.

Success criteria:

- user can create an alien language whose grammar depends on non-speech modality;
- system explains why its structure differs from human vocal language;
- alien utterances can be represented visually or structurally.

### Phase 12: LLM Copilot

**Goal:** add AI assistance without undermining formal correctness.

Tasks:

- define prompt templates grounded in language genome JSON;
- implement “explain this trace”;
- implement “suggest plausible sound changes”;
- implement “generate grammar sketch from structured data”;
- implement “flag typological oddities”;
- implement “propose alien embodiment constraints.”

Success criteria:

- AI explanations cite the formal data;
- AI suggestions can be accepted/rejected as structured changes;
- no freeform AI output mutates the language without validation.

---

## 14. Milestone Plan

### Milestone 1: Root Generator

**Deliverable:** CLI generates valid proto-language roots from a phoneme inventory and phonotactic template.

Estimated scope:

- 2–4 crates;
- no UI;
- no database;
- no sound-change rules yet.

Demo:

```text
stemma new proto_asterian
stemma generate-roots --count 100 --seed 42
stemma export-md proto_asterian.md
```

### Milestone 2: Sound Change MVP

**Deliverable:** CLI applies ordered rules and produces traces.

Demo:

```text
stemma apply-rules proto_asterian rules/coastal.stemrule --out coastal_asterian
stemma trace-word coastal_asterian water
```

### Milestone 3: Language Family MVP

**Deliverable:** proto-language forks into daughter languages with cognate tables.

Demo:

```text
stemma fork proto_asterian coastal highland riverine
stemma apply-rules coastal rules/coastal.stemrule
stemma apply-rules highland rules/highland.stemrule
stemma cognates --meanings water sun star king mother
```

### Milestone 4: Markdown Portfolio Demo

**Deliverable:** a polished generated document showing a mini language family.

Output sections:

- proto-language sketch;
- daughter-language summaries;
- sound-change histories;
- cognate tables;
- five etymology traces;
- notes on plausibility.

### Milestone 5: Visual Explorer

**Deliverable:** basic desktop/web UI with lexicon, timeline, cognate table, and word trace.

### Milestone 6: Morphology and Semantic Drift

**Deliverable:** simple affix systems, paradigm tables, and meaning shifts.

### Milestone 7: Script Evolution

**Deliverable:** orthography, glyph ancestry, and exportable inscriptions.

### Milestone 8: Alien Modality Lab

**Deliverable:** first non-human communication channel prototype.

---

## 15. First Tickets for Claude Code

Use small, testable tasks.

### Ticket 1: Workspace Setup

```text
Create a Rust workspace for Stemma with crates stem_core, stem_phonology, stem_lexicon, stem_soundchange, stem_io, and stem_cli. Add serde, thiserror, anyhow, rand, and clap where appropriate. Add a top-level README and make cargo test pass.
```

### Ticket 2: ID and Error Types

```text
Implement stable typed IDs for LanguageId, WordId, PhonemeId, RuleId, and CognateSetId. Use newtype wrappers around UUID or strings. Add basic StemmaError enum with thiserror.
```

### Ticket 3: Phoneme Inventory

```text
Implement SegmentKind, FeatureBundle, Phoneme, and PhonemeInventory. Include validation that phoneme IDs are unique and IPA strings are non-empty. Add tests.
```

### Ticket 4: Phonotactic Root Generator

```text
Implement a simple phonotactic template generator supporting C, V, optional groups, and weighted phoneme selection. Generate deterministic roots from a seed. Add tests for reproducibility.
```

### Ticket 5: Lexicon Model

```text
Implement WordEntry, Lexicon, Gloss, PartOfSpeech, and CognateSetId. Add JSON/RON serialization. Add tests for round-trip serialization.
```

### Ticket 6: Sound Change Rule Model

```text
Implement SoundChangeRule with target pattern, replacement action, environment, and name. Do not build a DSL parser yet. Use Rust structs directly. Add tests applying simple rules to phonological forms.
```

### Ticket 7: Rule Application Trace

```text
Add EvolutionTrace and RuleApplicationTrace. Every sound change should record input, output, rule name, matched spans, and changed segments. Add tests.
```

### Ticket 8: Language Forking

```text
Implement LanguageGenome and fork_language(parent, child_name). Fork should preserve cognate IDs and record parent ID. Add tests.
```

### Ticket 9: Cognate Table Export

```text
Implement a function that builds a cognate table across multiple related languages by cognate set and gloss. Export to Markdown.
```

### Ticket 10: CLI Demo Command

```text
Create a `stemma demo` command that generates a proto-language, forks it into two daughter languages, applies hardcoded rules, and writes a Markdown report to output/demo.md.
```

---

## 16. Testing Strategy

### 16.1 Unit Tests

Test core formal logic:

- phoneme inventory validation;
- feature matching;
- phonotactic generation;
- root generation reproducibility;
- sound-change application;
- environment matching;
- chained sound changes;
- trace generation;
- lexicon serialization;
- cognate preservation;
- language forking.

### 16.2 Golden Tests

Maintain fixture languages with expected outputs.

Example:

```text
fixtures/
  proto_simple.ron
  rules_simple.ron
  expected_coastal_lexicon.csv
  expected_trace_water.json
```

Golden tests should catch accidental changes in rule behavior.

### 16.3 Property Tests

Use property testing where useful.

Examples:

- generated roots contain only inventory phonemes;
- applying zero rules leaves lexicon unchanged;
- forked language preserves all cognate-set IDs;
- every trace output’s final form equals the stored word form;
- deleting a phoneme never creates invalid segment references.

### 16.4 Snapshot Tests for Export

Markdown exports can be snapshot-tested to prevent formatting regressions.

### 16.5 Scientific Sanity Tests

Add tests that check broad plausibility constraints:

- no generated language has an empty vowel inventory unless explicitly alien/non-vocal;
- every spoken human-mode language has at least one syllable nucleus type;
- no language has impossible references to missing phonemes;
- sound-change rules report when they never apply;
- daughter languages preserve lineage metadata.

---

## 17. Plausibility Scoring

The system should not label languages simply valid or invalid. It should produce plausibility profiles.

Example:

```text
Naturalistic human plausibility: 82%
Typological rarity: high
Phonotactic complexity: moderate
Morphological irregularity: low
Historical depth: strong
Script-history coherence: weak
Alien embodiment dependence: none
```

Possible warnings:

```text
This language has 80 consonants and 2 vowels. This is possible as a speculative design but typologically unusual.

This language has SOV order, prepositions, noun-genitive order, and no case marking. This combination is not impossible but deserves a historical explanation.

This descendant language has extreme irregularity after only 100 years of simulated change. Consider increasing time depth, contact intensity, or morphological erosion.

This script becomes alphabetic directly from pictograms with no rebus or syllabic intermediate stage. Consider adding a transitional phase.
```

Plausibility scoring should be transparent, not authoritarian. The tool should guide the creator, not police them.

---

## 18. Alien Language Design Model

Alien language support should not mean “random impossible sounds.” It should mean communication systems shaped by alien bodies and environments.

### 18.1 Embodiment Profile

```rust
pub struct EmbodimentProfile {
    pub vocal_tract: Option<VocalTractProfile>,
    pub auditory_range: Option<FrequencyRange>,
    pub visual_channels: Vec<VisualChannel>,
    pub chemical_channels: Vec<ChemicalChannel>,
    pub tactile_channels: Vec<TactileChannel>,
    pub electric_or_magnetic_channels: Vec<FieldChannel>,
    pub manipulators: Vec<ManipulatorProfile>,
    pub social_cognition: SocialCognitionProfile,
    pub environment: EnvironmentProfile,
}
```

### 18.2 Channel Constraints

Each channel has constraints:

```text
bandwidth
persistence
directionality
range
simultaneity
noise profile
privacy
energy cost
learnability
cultural salience
```

A chemical language differs from an ultrasonic language because scent persists, diffuses, and mixes. A visual pulse language differs from human speech because it may encode multiple simultaneous parameters. A hive harmonic language may require multiple speakers to form a complete utterance.

### 18.3 Alien Grammar Consequences

Examples:

- persistent scent channels encourage evidential marking, source tracking, territorial grammar, and delayed interpretation;
- color-pulse channels encourage simultaneous morphology, mood coloring, and spatial argument indexing;
- tentacular gesture channels encourage classifier systems and poly-manual agreement;
- electrical field channels encourage contact/proximity grammar and body-orientation deixis;
- hive communication encourages distributed pronouns, plural cognition, and subgroup indexing.

---

## 19. Export Formats

### 19.1 Markdown Export

Primary human-readable format.

Sections:

```text
Language overview
Phoneme inventory
Phonotactics
Prosody
Sound changes
Morphology sketch
Syntax sketch
Lexicon
Cognate tables
Etymology traces
Writing system
Historical notes
Plausibility report
```

### 19.2 JSON/RON Export

Primary machine-readable project format.

### 19.3 CSV Export

Useful for dictionaries and cognate tables.

### 19.4 HTML Export

Later, generate browsable mini-sites for language families.

### 19.5 CLDF-Inspired Export

Long-term: support CLDF-inspired tables for interoperability with linguistic tooling. Full compliance can come later.

---

## 20. Risks and Mitigations

### 20.1 Scope Explosion

This project can easily become a conlang tool, linguistic simulator, alien biology tool, script designer, grammar parser, LLM product, and worldbuilding suite all at once.

Mitigation:

- protect the MVP;
- implement the diachronic lexicon first;
- avoid UI polish until the engine works;
- postpone alien modality until human-mode language evolution is solid.

### 20.2 Linguistic Overreach

The project may accidentally encode simplistic or false linguistic assumptions.

Mitigation:

- use transparent plausibility warnings rather than hard claims;
- document assumptions;
- allow users to override defaults;
- ground defaults in typological references;
- separate “common,” “rare,” “speculative,” and “alien” modes.

### 20.3 LLM Hallucination

An LLM copilot may invent explanations not supported by the formal model.

Mitigation:

- require AI output to cite structured data;
- treat suggestions as proposals, not mutations;
- validate all accepted changes;
- keep deterministic traces as source of truth.

### 20.4 Rule DSL Complexity

A powerful sound-change DSL can become hard to learn.

Mitigation:

- start with struct-based rules;
- add a readable DSL later;
- provide examples and templates;
- include rule tracing and “why did this not apply?” diagnostics.

### 20.5 UI Complexity

The visual interface could consume all development time.

Mitigation:

- build CLI first;
- use Markdown export as early proof;
- add UI only after engine MVP works;
- keep views read-only before adding full editing.

---

## 21. The Portfolio Demo

The strongest public demo should be called something like:

**“Growing a Language Family in 90 Seconds.”**

Demo flow:

1. Generate Proto-Asterian with 200 roots.
2. Fork into Coastal, Highland, and Riverine branches.
3. Apply different sound-change histories.
4. Show cognate table.
5. Click `star` and trace its evolution.
6. Show that `star` became “omen” in Coastal but remained “star” in Highland.
7. Export a mini grammar and dictionary.
8. Optionally show a script evolution teaser.

Example exported snippet:

```text
Proto-Asterian *takala “star, bright thing”
  Coastal taal “omen, royal sign”
  Highland tazal “star”
  Riverine tala “night-signal”
```

This demo communicates the whole project: formal rules, beauty, history, story, and scientific discipline.

---

## 22. Long-Term Vision

The final version of Stemma should feel like a microscope for imaginary languages.

A writer could create an ancient language family for a fantasy empire and trace why the river people’s word for “king” is cognate with the mountain people’s word for “storm.” A game developer could generate regionally coherent names, inscriptions, dialects, and factions. A linguistics student could explore how sound changes create irregularity. A sci-fi author could design a language for a species that communicates in ultraviolet skin patterns and chemical memory trails.

The project’s deepest value is not automation. It is constrained imagination. It lets users create fictional languages that have weight, ancestry, residue, mutation, and history.

Bad generators create random syllables.

Good conlang tools apply rules.

Stemma should grow languages.

---

## 23. Immediate Next Step

Build the smallest serious kernel:

```text
One proto-language.
One phoneme inventory.
One root generator.
One lexicon.
One sound-change rule engine.
One fork.
One cognate table.
One word trace.
One Markdown export.
```

That is enough to prove the idea.

Everything else grows from there.
