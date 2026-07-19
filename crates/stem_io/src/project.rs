//! Reading and writing project files, format chosen by extension.

use std::fmt;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use stem_core::{Result, StemmaError};

/// RON options used when **reading**.
///
/// `IMPLICIT_SOME` lets an authored file write `romanization: "y"` instead of
/// `romanization: Some("y")`. Language files are written by hand — a conlanger
/// editing an inventory should not have to know that a field is an `Option` in
/// the Rust type behind it.
///
/// Enabling it here, as a *default* extension, means a fixture parses whether or
/// not it declares `#![enable(implicit_some)]` at the top. Reading is lenient.
fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
}

/// Pretty-printing config used when **writing**.
///
/// Writing goes through plain [`ron::ser::to_string_pretty`] rather than
/// [`ron::Options`] on purpose. ron only emits the `#![enable(...)]` header for
/// extensions that are *not* already defaults of the `Options` doing the
/// serialising — so routing writes through `ron_options()` would silently produce
/// files that only Stemma could read. Writing is explicit: the file declares what
/// it relies on, and any plain `ron::from_str` can consume it.
fn ron_pretty() -> ron::ser::PrettyConfig {
    ron::ser::PrettyConfig::default().extensions(ron::extensions::Extensions::IMPLICIT_SOME)
}

/// A supported on-disk project format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Rusty Object Notation — the primary authored format.
    Ron,
    /// JSON — for interchange with the UI and other tools.
    Json,
}

impl Format {
    /// Infers the format from a path's extension.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("ron") => Ok(Self::Ron),
            Some("json") => Ok(Self::Json),
            _ => Err(StemmaError::UnsupportedFormat {
                path: path.display().to_string(),
            }),
        }
    }

    /// The format's name, as it appears in error messages.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ron => "RON",
            Self::Json => "JSON",
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Loads a value from a file, choosing the parser by extension.
pub fn load<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let format = Format::from_path(path)?;
    let text = std::fs::read_to_string(path).map_err(|e| StemmaError::io("read", path, e))?;

    load_str(&text, format).map_err(|err| match err {
        // `load_str` has no path to report; attach it here so the user is told
        // *which* file failed, which is the only useful part of a parse error.
        StemmaError::Parse { message, .. } => StemmaError::Parse {
            path: path.display().to_string(),
            format: format.name(),
            message,
        },
        other => other,
    })
}

/// Parses a value from a string in the given format.
pub fn load_str<T: DeserializeOwned>(text: &str, format: Format) -> Result<T> {
    let to_parse_error = |message: String| StemmaError::Parse {
        path: "<string>".to_owned(),
        format: format.name(),
        message,
    };

    match format {
        Format::Ron => ron_options()
            .from_str(text)
            .map_err(|e| to_parse_error(e.to_string())),
        Format::Json => serde_json::from_str(text).map_err(|e| to_parse_error(e.to_string())),
    }
}

/// Serialises a value to a string in the given format, pretty-printed.
///
/// Output is always pretty-printed: project files are read, diffed, and reviewed
/// by hand, and a one-line blob would make a fork diff unreadable.
pub fn to_string<T: Serialize>(value: &T, format: Format) -> Result<String> {
    let to_serialize_error = |message: String| StemmaError::Serialize {
        format: format.name(),
        message,
    };

    match format {
        Format::Ron => ron::ser::to_string_pretty(value, ron_pretty())
            .map_err(|e| to_serialize_error(e.to_string())),
        Format::Json => {
            serde_json::to_string_pretty(value).map_err(|e| to_serialize_error(e.to_string()))
        }
    }
}

/// Writes a value to a file, choosing the format by extension.
///
/// Creates parent directories if they are missing, so callers can write to
/// `output/demo/coastal.ron` without preparing the tree first.
pub fn save<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    let path = path.as_ref();
    let format = Format::from_path(path)?;
    let mut text = to_string(value, format)?;
    text.push('\n');

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| StemmaError::io("create directory for", path, e))?;
    }

    std::fs::write(path, text).map_err(|e| StemmaError::io("write", path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_genome::LanguageGenome;
    use stem_phonology::{Phoneme, PhonemeInventory, SegmentKind};

    fn genome() -> LanguageGenome {
        LanguageGenome::proto("proto_asterian", "Proto-Asterian")
            .with_seed(42)
            .with_phonemes(PhonemeInventory::from_phonemes([
                Phoneme::new("ph_t", "t", SegmentKind::Consonant),
                Phoneme::new("ph_a", "a", SegmentKind::Vowel).with_weight(25),
                Phoneme::new("ph_sh", "ʃ", SegmentKind::Consonant).with_romanization("sh"),
            ]))
    }

    #[test]
    fn format_is_inferred_from_the_extension() {
        assert_eq!(Format::from_path("a/b.ron").unwrap(), Format::Ron);
        assert_eq!(Format::from_path("a/b.json").unwrap(), Format::Json);
        assert_eq!(Format::from_path("a/b.RON").unwrap(), Format::Ron);
        assert!(Format::from_path("a/b.txt").is_err());
        assert!(Format::from_path("noextension").is_err());
    }

    #[test]
    fn ron_round_trips_a_genome_unchanged() {
        let original = genome();
        let text = to_string(&original, Format::Ron).expect("serialise");
        let back: LanguageGenome = load_str(&text, Format::Ron).expect("deserialise");
        assert_eq!(back, original);
    }

    #[test]
    fn json_round_trips_a_genome_unchanged() {
        let original = genome();
        let text = to_string(&original, Format::Json).expect("serialise");
        let back: LanguageGenome = load_str(&text, Format::Json).expect("deserialise");
        assert_eq!(back, original);
    }

    #[test]
    fn the_two_formats_carry_identical_data() {
        // RON is authored, JSON is interchange; a project must survive the trip.
        let original = genome();
        let via_ron: LanguageGenome =
            load_str(&to_string(&original, Format::Ron).unwrap(), Format::Ron).unwrap();
        let via_json: LanguageGenome =
            load_str(&to_string(&original, Format::Json).unwrap(), Format::Json).unwrap();
        assert_eq!(via_ron, via_json);
    }

    #[test]
    fn non_ascii_ipa_survives_serialisation() {
        // IPA is the whole point; a mangled /ʃ/ would be silent data loss.
        let text = to_string(&genome(), Format::Ron).expect("serialise");
        let back: LanguageGenome = load_str(&text, Format::Ron).expect("deserialise");
        assert_eq!(back.phonemes.get(&"ph_sh".into()).unwrap().ipa, "ʃ");
    }

    #[test]
    fn authored_ron_may_omit_some_around_optional_fields() {
        // The ergonomic reason RON was chosen over JSON. If this regresses, every
        // hand-written fixture with a romanisation stops loading.
        let text = r#"(
            id: "t", name: "Test",
            phonemes: [ (id: "ph_sh", ipa: "ʃ", romanization: "sh", kind: consonant) ],
        )"#;
        let genome: LanguageGenome = load_str(text, Format::Ron).expect("implicit Some must parse");
        assert_eq!(
            genome
                .phonemes
                .get(&"ph_sh".into())
                .unwrap()
                .romanization
                .as_deref(),
            Some("sh")
        );
    }

    #[test]
    fn ron_output_declares_the_extensions_it_relies_on() {
        // Files Stemma writes must be readable by a plain `ron::from_str`, not
        // only by this crate's configured loader.
        let text = to_string(&genome(), Format::Ron).expect("serialise");
        assert!(
            text.contains("implicit_some"),
            "missing extension header:\n{text}"
        );
        let back: LanguageGenome = ron::from_str(&text).expect("plain ron must parse our output");
        assert_eq!(back, genome());
    }

    #[test]
    fn a_parse_error_names_the_file_and_the_format() {
        let dir = std::env::temp_dir().join("stemma_test_parse_error");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("broken.ron");
        std::fs::write(&path, "this is not ron(((").unwrap();

        let err = load::<LanguageGenome>(&path).expect_err("should fail to parse");
        let message = err.to_string();
        assert!(message.contains("broken.ron"), "{message}");
        assert!(message.contains("RON"), "{message}");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_file_reports_the_path() {
        let err = load::<LanguageGenome>("does/not/exist.ron").expect_err("should fail");
        assert!(err.to_string().contains("does/not/exist.ron"), "{err}");
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let dir = std::env::temp_dir().join("stemma_test_save_nested");
        std::fs::remove_dir_all(&dir).ok();
        let path = dir.join("deep").join("proto.ron");

        save(&path, &genome()).expect("save");
        let back: LanguageGenome = load(&path).expect("load");
        assert_eq!(back, genome());

        std::fs::remove_dir_all(&dir).ok();
    }
}
