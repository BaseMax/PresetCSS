use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::{did_you_mean, locate, Diagnostic};

pub type TokenMap = IndexMap<String, String>;

#[derive(Debug, Default, Deserialize)]
pub struct RawPreset {
    #[serde(default)]
    pub options: Options,

    pub colors: Option<TokenMap>,
    pub spacing: Option<TokenMap>,
    pub sizes: Option<TokenMap>,
    pub fonts: Option<TokenMap>,
    pub text: Option<TokenMap>,
    pub radius: Option<TokenMap>,
    pub shadows: Option<TokenMap>,
    pub breakpoints: Option<TokenMap>,

    #[serde(default)]
    pub extend: Extend,
    pub variants: Option<IndexMap<String, Vec<String>>>,
    #[serde(default)]
    pub utilities: TokenMap,

    #[serde(flatten)]
    pub unknown: IndexMap<String, toml::Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Extend {
    #[serde(default)]
    pub colors: TokenMap,
    #[serde(default)]
    pub spacing: TokenMap,
    #[serde(default)]
    pub sizes: TokenMap,
    #[serde(default)]
    pub fonts: TokenMap,
    #[serde(default)]
    pub text: TokenMap,
    #[serde(default)]
    pub radius: TokenMap,
    #[serde(default)]
    pub shadows: TokenMap,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Options {
    #[serde(default = "default_output")]
    pub output: String,
    #[serde(default)]
    pub content: Vec<String>,
    #[serde(default)]
    pub minify: bool,
    #[serde(default = "default_base")]
    pub base: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub source_comments: bool,
    #[serde(default = "default_true")]
    pub dedup: bool,

    #[serde(flatten)]
    pub unknown: IndexMap<String, toml::Value>,
}

fn default_output() -> String {
    "dist/preset.css".into()
}
fn default_base() -> String {
    "minimal".into()
}
fn default_true() -> bool {
    true
}

impl Default for Options {
    fn default() -> Self {
        Options {
            output: default_output(),
            content: Vec::new(),
            minify: false,
            base: default_base(),
            prefix: String::new(),
            source_comments: false,
            dedup: true,
            unknown: IndexMap::new(),
        }
    }
}

pub const KNOWN_GROUPS: &[&str] = &[
    "options",
    "colors",
    "spacing",
    "sizes",
    "fonts",
    "text",
    "radius",
    "shadows",
    "breakpoints",
    "extend",
    "variants",
    "utilities",
];

const KNOWN_OPTIONS: &[&str] = &[
    "output",
    "content",
    "minify",
    "base",
    "prefix",
    "source_comments",
    "dedup",
];

pub fn parse(src: &str) -> Result<(RawPreset, Vec<Diagnostic>), Vec<Diagnostic>> {
    let raw: RawPreset = toml::from_str(src).map_err(|e| {
        let mut diag = Diagnostic::error(format!("invalid TOML: {}", e.message()));
        if let Some(span) = e.span() {
            let line = src[..span.start.min(src.len())].lines().count();
            diag = diag.at(line.max(1));
        }
        vec![diag]
    })?;

    let mut diags = Vec::new();
    for key in raw.unknown.keys() {
        let mut d = Diagnostic::error(format!("unknown table [{key}]"));
        if let Some(line) = locate(src, "", key) {
            d = d.at(line);
        }
        if let Some(s) = did_you_mean(key, KNOWN_GROUPS.iter().copied()) {
            d = d.with_help(format!("did you mean [{s}]?"));
        }
        diags.push(d);
    }
    for key in raw.options.unknown.keys() {
        let mut d = Diagnostic::error(format!("unknown option '{key}'"));
        if let Some(line) = locate(src, "options", key) {
            d = d.at(line);
        }
        if let Some(s) = did_you_mean(key, KNOWN_OPTIONS.iter().copied()) {
            d = d.with_help(format!("did you mean '{s}'?"));
        }
        diags.push(d);
    }

    Ok((raw, diags))
}

pub fn validate_color(value: &str) -> bool {
    let v = value.trim();
    if let Some(hex) = v.strip_prefix('#') {
        let ok_len = matches!(hex.len(), 3 | 4 | 6 | 8);
        return ok_len && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    !v.is_empty()
}
