use std::collections::HashSet;

use crate::theme::{Group, ResolvedTheme};

#[derive(Debug, Clone)]
pub struct Decl {
    pub prop: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub class: String,
    pub pseudos: String,
    pub media: Option<String>,
    pub decls: Vec<Decl>,
    pub src: String,
}

impl Rule {
    pub fn selector(&self) -> String {
        format!(".{}{}", escape_class(&self.class), self.pseudos)
    }
}

struct Generator {
    group: Group,
    prefix: &'static str,
    props: &'static [&'static str],
}

const GENERATORS: &[Generator] = &[
    Generator { group: Group::Colors, prefix: "bg", props: &["background-color"] },
    Generator { group: Group::Colors, prefix: "text", props: &["color"] },
    Generator { group: Group::Colors, prefix: "border", props: &["border-color"] },

    Generator { group: Group::Spacing, prefix: "p", props: &["padding"] },
    Generator { group: Group::Spacing, prefix: "px", props: &["padding-left", "padding-right"] },
    Generator { group: Group::Spacing, prefix: "py", props: &["padding-top", "padding-bottom"] },
    Generator { group: Group::Spacing, prefix: "pt", props: &["padding-top"] },
    Generator { group: Group::Spacing, prefix: "pr", props: &["padding-right"] },
    Generator { group: Group::Spacing, prefix: "pb", props: &["padding-bottom"] },
    Generator { group: Group::Spacing, prefix: "pl", props: &["padding-left"] },

    Generator { group: Group::Spacing, prefix: "m", props: &["margin"] },
    Generator { group: Group::Spacing, prefix: "mx", props: &["margin-left", "margin-right"] },
    Generator { group: Group::Spacing, prefix: "my", props: &["margin-top", "margin-bottom"] },
    Generator { group: Group::Spacing, prefix: "mt", props: &["margin-top"] },
    Generator { group: Group::Spacing, prefix: "mr", props: &["margin-right"] },
    Generator { group: Group::Spacing, prefix: "mb", props: &["margin-bottom"] },
    Generator { group: Group::Spacing, prefix: "ml", props: &["margin-left"] },

    Generator { group: Group::Spacing, prefix: "gap", props: &["gap"] },

    Generator { group: Group::Sizes, prefix: "w", props: &["width"] },
    Generator { group: Group::Sizes, prefix: "h", props: &["height"] },
    Generator { group: Group::Sizes, prefix: "max-w", props: &["max-width"] },
    Generator { group: Group::Sizes, prefix: "min-h", props: &["min-height"] },

    Generator { group: Group::Fonts, prefix: "font", props: &["font-family"] },
    Generator { group: Group::Text, prefix: "text", props: &["font-size"] },

    Generator { group: Group::Radius, prefix: "rounded", props: &["border-radius"] },
    Generator { group: Group::Shadows, prefix: "shadow", props: &["box-shadow"] },
];

pub const STATICS: &[(&str, &[(&str, &str)])] = &[
    ("flex", &[("display", "flex")]),
    ("inline-flex", &[("display", "inline-flex")]),
    ("grid", &[("display", "grid")]),
    ("block", &[("display", "block")]),
    ("inline-block", &[("display", "inline-block")]),
    ("inline", &[("display", "inline")]),
    ("hidden", &[("display", "none")]),
    ("flex-row", &[("flex-direction", "row")]),
    ("flex-col", &[("flex-direction", "column")]),
    ("flex-wrap", &[("flex-wrap", "wrap")]),
    ("items-start", &[("align-items", "flex-start")]),
    ("items-center", &[("align-items", "center")]),
    ("items-end", &[("align-items", "flex-end")]),
    ("justify-start", &[("justify-content", "flex-start")]),
    ("justify-center", &[("justify-content", "center")]),
    ("justify-between", &[("justify-content", "space-between")]),
    ("justify-end", &[("justify-content", "flex-end")]),
    ("text-left", &[("text-align", "left")]),
    ("text-center", &[("text-align", "center")]),
    ("text-right", &[("text-align", "right")]),
    ("font-bold", &[("font-weight", "700")]),
    ("font-normal", &[("font-weight", "400")]),
    ("italic", &[("font-style", "italic")]),
    ("underline", &[("text-decoration-line", "underline")]),
    ("static", &[("position", "static")]),
    ("relative", &[("position", "relative")]),
    ("absolute", &[("position", "absolute")]),
    ("fixed", &[("position", "fixed")]),
    ("sticky", &[("position", "sticky")]),
    ("w-full", &[("width", "100%")]),
    ("h-full", &[("height", "100%")]),
    ("overflow-hidden", &[("overflow", "hidden")]),
    ("overflow-auto", &[("overflow", "auto")]),
    ("rounded-full", &[("border-radius", "9999px")]),
    ("border", &[("border-width", "1px"), ("border-style", "solid")]),
    ("cursor-pointer", &[("cursor", "pointer")]),
];

pub const STATE_PSEUDOS: &[(&str, &str)] = &[
    ("hover", ":hover"),
    ("focus", ":focus"),
    ("focus-visible", ":focus-visible"),
    ("active", ":active"),
    ("disabled", ":disabled"),
    ("first", ":first-child"),
    ("last", ":last-child"),
];

fn pseudo_for(state: &str) -> Option<&'static str> {
    STATE_PSEUDOS.iter().find(|(s, _)| *s == state).map(|(_, p)| *p)
}

pub fn escape_class(class: &str) -> String {
    let mut out = String::with_capacity(class.len() + 2);
    for ch in class.chars() {
        if matches!(ch, ':' | '.' | '/') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

struct BaseRule {
    group_key: String,
    class: String,
    decls: Vec<Decl>,
    src: String,
}

fn collect_base(theme: &ResolvedTheme) -> Vec<BaseRule> {
    let p = &theme.options.prefix;
    let mut base = Vec::new();

    for g in GENERATORS {
        for (key, value) in theme.group(g.group) {
            base.push(BaseRule {
                group_key: g.group.key().to_string(),
                class: format!("{p}{}-{}", g.prefix, key),
                decls: g
                    .props
                    .iter()
                    .map(|prop| Decl { prop: prop.to_string(), value: value.clone() })
                    .collect(),
                src: format!("{}.{}", g.group.key(), key),
            });
        }
    }

    for (class, decls) in STATICS {
        base.push(BaseRule {
            group_key: "static".to_string(),
            class: format!("{p}{class}"),
            decls: decls
                .iter()
                .map(|(prop, value)| Decl { prop: prop.to_string(), value: value.to_string() })
                .collect(),
            src: format!("static.{class}"),
        });
    }

    for (name, css) in &theme.utilities {
        base.push(BaseRule {
            group_key: "utilities".to_string(),
            class: format!("{p}{name}"),
            decls: parse_raw_decls(css),
            src: format!("utilities.{name}"),
        });
    }

    base
}

fn parse_raw_decls(css: &str) -> Vec<Decl> {
    css.split(';')
        .filter_map(|chunk| {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                return None;
            }
            let (prop, value) = chunk.split_once(':')?;
            Some(Decl { prop: prop.trim().to_string(), value: value.trim().to_string() })
        })
        .collect()
}

pub fn generate(theme: &ResolvedTheme, used: Option<&HashSet<String>>) -> Vec<Rule> {
    let base = collect_base(theme);
    let mut out = Vec::new();

    let keep = |class: &str| used.is_none_or(|set| set.contains(class));

    for br in &base {
        let variants = theme.variants_for(&br.group_key);
        let states: Vec<&str> = variants
            .iter()
            .map(String::as_str)
            .filter(|v| pseudo_for(v).is_some())
            .collect();
        let responsive = variants.iter().any(|v| v == "responsive");

        if keep(&br.class) {
            out.push(Rule {
                class: br.class.clone(),
                pseudos: String::new(),
                media: None,
                decls: br.decls.clone(),
                src: br.src.clone(),
            });
        }

        for st in &states {
            let class = format!("{st}:{}", br.class);
            if keep(&class) {
                out.push(Rule {
                    class,
                    pseudos: pseudo_for(st).unwrap().to_string(),
                    media: None,
                    decls: br.decls.clone(),
                    src: br.src.clone(),
                });
            }
        }

        if responsive {
            for (bp_name, bp_val) in &theme.breakpoints {
                let class = format!("{bp_name}:{}", br.class);
                if keep(&class) {
                    out.push(Rule {
                        class,
                        pseudos: String::new(),
                        media: Some(bp_val.clone()),
                        decls: br.decls.clone(),
                        src: br.src.clone(),
                    });
                }
                for st in &states {
                    let class = format!("{bp_name}:{st}:{}", br.class);
                    if keep(&class) {
                        out.push(Rule {
                            class,
                            pseudos: pseudo_for(st).unwrap().to_string(),
                            media: Some(bp_val.clone()),
                            decls: br.decls.clone(),
                            src: br.src.clone(),
                        });
                    }
                }
            }
        }
    }

    out
}

pub fn class_universe(theme: &ResolvedTheme) -> Vec<String> {
    generate(theme, None).into_iter().map(|r| r.class).collect()
}
