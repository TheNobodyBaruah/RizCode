//! Language detection and per-language naming rules.
//!
//! The sandbox supports exactly three languages. Everything the rest of the crate needs to know
//! about a language — its source file name, its extension, its label and the interpreter used to
//! run it — lives here so there is a single source of truth.

use std::path::Path;

/// A language the sandbox can execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Compiled with `rustc`, then executed as a native binary.
    Rust,
    /// Executed by a CPython interpreter.
    Python,
    /// Executed by Node.js.
    JavaScript,
}

impl Language {
    /// Every supported language, in the order they are offered in the UI.
    pub const ALL: [Language; 3] = [Language::Rust, Language::Python, Language::JavaScript];

    /// Detects a language from a file extension (without the dot).
    ///
    /// The comparison is case-insensitive, so `RS`, `Rs` and `rs` all resolve to
    /// [`Language::Rust`].
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.trim().trim_start_matches('.').to_ascii_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "py" | "pyw" => Some(Language::Python),
            "js" | "mjs" | "cjs" => Some(Language::JavaScript),
            _ => None,
        }
    }

    /// Detects a language from a path's extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Language::from_extension)
    }

    /// The name of the source file written into the throwaway workspace.
    pub fn file_name(&self) -> &'static str {
        match self {
            Language::Rust => "main.rs",
            Language::Python => "main.py",
            Language::JavaScript => "main.js",
        }
    }

    /// The canonical extension of the language.
    pub fn extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::JavaScript => "js",
        }
    }

    /// Human readable label shown in the UI.
    pub fn label(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
        }
    }

    /// Parses a language from its serialised (lower case) name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "python" | "py" => Some(Language::Python),
            "javascript" | "js" | "node" => Some(Language::JavaScript),
            _ => None,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_rust_from_extension() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
    }

    #[test]
    fn detects_python_from_extension() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
    }

    #[test]
    fn detects_javascript_from_extensions() {
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("mjs"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("cjs"), Some(Language::JavaScript));
    }

    #[test]
    fn extension_detection_is_case_insensitive() {
        assert_eq!(Language::from_extension("RS"), Some(Language::Rust));
        assert_eq!(Language::from_extension("Py"), Some(Language::Python));
        assert_eq!(Language::from_extension("JS"), Some(Language::JavaScript));
    }

    #[test]
    fn unknown_extension_is_none() {
        assert_eq!(Language::from_extension("txt"), None);
        assert_eq!(Language::from_extension(""), None);
        assert_eq!(Language::from_extension("ruby"), None);
    }

    #[test]
    fn detects_language_from_path() {
        assert_eq!(
            Language::from_path(&PathBuf::from("/tmp/demo.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("/tmp/demo.py")),
            Some(Language::Python)
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("/tmp/demo.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(Language::from_path(&PathBuf::from("/tmp/demo")), None);
    }

    #[test]
    fn file_names_and_extensions_match_the_plan() {
        assert_eq!(Language::Rust.file_name(), "main.rs");
        assert_eq!(Language::Python.file_name(), "main.py");
        assert_eq!(Language::JavaScript.file_name(), "main.js");
        assert_eq!(Language::Rust.extension(), "rs");
        assert_eq!(Language::Python.extension(), "py");
        assert_eq!(Language::JavaScript.extension(), "js");
    }

    #[test]
    fn labels_are_human_readable() {
        assert_eq!(Language::Rust.label(), "Rust");
        assert_eq!(Language::Python.label(), "Python");
        assert_eq!(Language::JavaScript.label(), "JavaScript");
    }

    #[test]
    fn round_trips_through_serialised_name() {
        for language in Language::ALL {
            assert_eq!(Language::from_name(language.label()), Some(language));
        }
        assert_eq!(Language::from_name("nope"), None);
    }
}
