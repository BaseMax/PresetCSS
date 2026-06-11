pub mod emit;
pub mod error;
pub mod generate;
pub mod preset;
#[cfg(feature = "scan")]
pub mod scan;
pub mod theme;

pub use error::Diagnostic;
pub use generate::{generate, Rule};
pub use theme::{resolve, ResolvedTheme, DEFAULT_PRESET};

use std::collections::HashSet;

pub fn compile_theme(src: &str) -> Result<ResolvedTheme, Vec<Diagnostic>> {
    let (raw, mut diags) = preset::parse(src)?;
    match resolve(raw, src) {
        Ok(theme) => {
            if diags.is_empty() {
                Ok(theme)
            } else {
                Err(diags)
            }
        }
        Err(mut errs) => {
            diags.append(&mut errs);
            Err(diags)
        }
    }
}

pub fn build_css(theme: &ResolvedTheme, used: Option<&HashSet<String>>) -> String {
    let rules = generate(theme, used);
    let opts = emit::EmitOptions {
        minify: theme.options.minify,
        source_comments: theme.options.source_comments,
        dedup: theme.options.dedup,
        base: theme.options.base.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    emit::emit_css(&rules, &opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme(src: &str) -> ResolvedTheme {
        compile_theme(src).expect("should compile")
    }

    #[test]
    fn default_preset_compiles() {
        let t = theme("");
        assert!(t.colors.contains_key("primary"));
        let css = build_css(&t, None);
        assert!(css.contains(".bg-primary"));
        assert!(css.contains("background-color: #3b82f6"));
    }

    #[test]
    fn color_generates_three_utilities() {
        let t = theme("[colors]\nbrand = \"#ff0000\"\n[variants]\ncolors = []\n");
        let css = build_css(&t, None);
        assert!(css.contains(".bg-brand"));
        assert!(css.contains(".text-brand"));
        assert!(css.contains(".border-brand"));
    }

    #[test]
    fn extend_adds_without_replacing() {
        let t = theme("[extend.colors]\nbrand = \"#ff0000\"\n");
        assert!(t.colors.contains_key("primary")); // default kept
        assert!(t.colors.contains_key("brand")); // extension added
    }

    #[test]
    fn replacing_group_drops_defaults() {
        let t = theme("[colors]\nonly = \"#fff\"\n");
        assert!(!t.colors.contains_key("primary"));
        assert!(t.colors.contains_key("only"));
    }

    #[test]
    fn hover_variant_opt_in() {
        let t = theme("[colors]\nbrand = \"#ff0000\"\n[variants]\ncolors = [\"hover\"]\n");
        let css = build_css(&t, None);
        assert!(css.contains(".hover\\:bg-brand:hover"));
    }

    #[test]
    fn responsive_uses_user_breakpoint_names() {
        let src = "[breakpoints]\ntablet = \"768px\"\n[variants]\nstatic = [\"responsive\"]\n";
        let t = theme(src);
        let css = build_css(&t, None);
        assert!(css.contains("@media (min-width:768px)"));
        assert!(css.contains(".tablet\\:flex"));
    }

    #[test]
    fn utilities_escape_hatch() {
        let t = theme("[utilities]\nbtn = \"padding: 1rem; color: white;\"\n");
        let css = build_css(&t, None);
        assert!(css.contains(".btn"));
        assert!(css.contains("padding: 1rem"));
        assert!(css.contains("color: white"));
    }

    #[test]
    fn collision_text_and_colors_errors() {
        let err = compile_theme("[colors]\nlg = \"#fff\"\n[text]\nlg = \"2rem\"\n").unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("both [colors] and [text]")));
    }

    #[test]
    fn bad_hex_is_rejected() {
        let err = compile_theme("[colors]\nx = \"#zzz\"\n").unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("invalid color")));
    }

    #[test]
    fn unknown_table_suggests() {
        let err = compile_theme("[colour]\nx = \"#fff\"\n").unwrap_err();
        let d = err.iter().find(|d| d.message.contains("unknown table")).unwrap();
        assert_eq!(d.help.as_deref(), Some("did you mean [colors]?"));
    }

    #[test]
    fn prefix_applies_after_variants() {
        let t = theme("[options]\nprefix = \"u-\"\n[colors]\nbrand = \"#f00\"\n[variants]\ncolors = [\"hover\"]\n");
        let css = build_css(&t, None);
        assert!(css.contains(".hover\\:u-bg-brand:hover"));
    }

    #[test]
    fn tree_shaking_keeps_only_used() {
        let t = theme("[colors]\na = \"#f00\"\nb = \"#0f0\"\n[variants]\ncolors=[]\n");
        let mut used = HashSet::new();
        used.insert("bg-a".to_string());
        let css = build_css(&t, Some(&used));
        assert!(css.contains(".bg-a"));
        assert!(!css.contains(".bg-b"));
        assert!(!css.contains(".text-a")); // not used
    }

    #[test]
    fn minify_has_no_newlines_in_rules() {
        let t = theme("[options]\nminify = true\n[colors]\nbrand=\"#f00\"\n[variants]\ncolors=[]\n");
        let css = build_css(&t, None);
        assert!(css.contains(".bg-brand{background-color:#f00}"));
    }

    #[test]
    fn deterministic_output() {
        let src = "[colors]\nbrand = \"#f00\"\n";
        assert_eq!(build_css(&theme(src), None), build_css(&theme(src), None));
    }
}
