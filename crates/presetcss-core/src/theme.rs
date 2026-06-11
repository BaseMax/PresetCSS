use indexmap::IndexMap;

use crate::error::{locate, Diagnostic};
use crate::preset::{parse, validate_color, Options, RawPreset, TokenMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    Colors,
    Spacing,
    Sizes,
    Fonts,
    Text,
    Radius,
    Shadows,
}

impl Group {
    pub fn key(self) -> &'static str {
        match self {
            Group::Colors => "colors",
            Group::Spacing => "spacing",
            Group::Sizes => "sizes",
            Group::Fonts => "fonts",
            Group::Text => "text",
            Group::Radius => "radius",
            Group::Shadows => "shadows",
        }
    }

    pub const ALL: [Group; 7] = [
        Group::Colors,
        Group::Spacing,
        Group::Sizes,
        Group::Fonts,
        Group::Text,
        Group::Radius,
        Group::Shadows,
    ];
}

#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    pub options: Options,
    pub colors: TokenMap,
    pub spacing: TokenMap,
    pub sizes: TokenMap,
    pub fonts: TokenMap,
    pub text: TokenMap,
    pub radius: TokenMap,
    pub shadows: TokenMap,
    pub breakpoints: TokenMap,
    pub variants: IndexMap<String, Vec<String>>,
    pub utilities: TokenMap,
}

impl ResolvedTheme {
    pub fn group(&self, g: Group) -> &TokenMap {
        match g {
            Group::Colors => &self.colors,
            Group::Spacing => &self.spacing,
            Group::Sizes => &self.sizes,
            Group::Fonts => &self.fonts,
            Group::Text => &self.text,
            Group::Radius => &self.radius,
            Group::Shadows => &self.shadows,
        }
    }

    pub fn variants_for(&self, key: &str) -> &[String] {
        self.variants.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

pub const DEFAULT_PRESET: &str = include_str!("../default.toml");

fn merged(base: Option<TokenMap>, default: Option<TokenMap>, extend: TokenMap) -> TokenMap {
    let mut out = base.or(default).unwrap_or_default();
    for (k, v) in extend {
        out.insert(k, v);
    }
    out
}

pub fn resolve(raw: RawPreset, src: &str) -> Result<ResolvedTheme, Vec<Diagnostic>> {
    let (defaults, _) = parse(DEFAULT_PRESET).expect("embedded default.toml must parse");

    let theme = ResolvedTheme {
        options: raw.options,
        colors: merged(raw.colors, defaults.colors, raw.extend.colors),
        spacing: merged(raw.spacing, defaults.spacing, raw.extend.spacing),
        sizes: merged(raw.sizes, defaults.sizes, raw.extend.sizes),
        fonts: merged(raw.fonts, defaults.fonts, raw.extend.fonts),
        text: merged(raw.text, defaults.text, raw.extend.text),
        radius: merged(raw.radius, defaults.radius, raw.extend.radius),
        shadows: merged(raw.shadows, defaults.shadows, raw.extend.shadows),
        breakpoints: raw.breakpoints.or(defaults.breakpoints).unwrap_or_default(),
        variants: raw.variants.or(defaults.variants).unwrap_or_default(),
        utilities: raw.utilities,
    };

    let mut diags = Vec::new();
    validate(&theme, src, &mut diags);
    if diags.is_empty() {
        Ok(theme)
    } else {
        Err(diags)
    }
}

fn validate(theme: &ResolvedTheme, src: &str, diags: &mut Vec<Diagnostic>) {
    for g in Group::ALL {
        for key in theme.group(g).keys() {
            if !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            {
                let mut d = Diagnostic::error(format!(
                    "invalid token key '{key}' in [{}] (allowed: A-Z a-z 0-9 . _ -)",
                    g.key()
                ));
                if let Some(line) = locate(src, g.key(), key) {
                    d = d.at(line);
                }
                diags.push(d);
            }
        }
    }

    for (key, value) in &theme.colors {
        if !validate_color(value) {
            let mut d =
                Diagnostic::error(format!("invalid color '{value}' for colors.{key}"));
            if let Some(line) = locate(src, "colors", key) {
                d = d.at(line);
            }
            diags.push(d.with_help("expected #hex, rgb()/hsl()/oklch(), or a CSS color keyword"));
        }
    }

    for key in theme.text.keys() {
        if theme.colors.contains_key(key) {
            let cl = locate(src, "colors", key);
            let tl = locate(src, "text", key);
            let mut d = Diagnostic::error(format!(
                "key '{key}' exists in both [colors] and [text]; both generate .text-{key}"
            ));
            if let (Some(c), Some(t)) = (cl, tl) {
                d = d.with_help(format!("see colors.{key} (line {c}) and text.{key} (line {t})"));
            }
            diags.push(d);
        }
    }

    for name in theme.breakpoints.keys() {
        if crate::generate::STATE_PSEUDOS.iter().any(|(s, _)| s == name) {
            diags.push(Diagnostic::error(format!(
                "breakpoint '{name}' collides with a reserved state variant name"
            )));
        }
    }
}
