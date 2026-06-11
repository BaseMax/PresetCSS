use std::collections::HashSet;
use std::path::PathBuf;

use aho_corasick::{AhoCorasick, MatchKind};
use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;

use crate::error::{did_you_mean, Diagnostic};

pub struct ScanResult {
    pub used: HashSet<String>,
    pub warnings: Vec<Diagnostic>,
}

pub fn scan(root: &std::path::Path, globs: &[String], universe: &[String]) -> Result<ScanResult, Vec<Diagnostic>> {
    let mut builder = GlobSetBuilder::new();
    for g in globs {
        let glob = Glob::new(g).map_err(|e| vec![Diagnostic::error(format!("invalid content glob '{g}': {e}"))])?;
        builder.add(glob);
    }
    let set = builder.build().map_err(|e| vec![Diagnostic::error(format!("glob set: {e}"))])?;

    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::Standard)
        .build(universe)
        .map_err(|e| vec![Diagnostic::error(format!("scanner build: {e}"))])?;

    let universe_set: HashSet<&str> = universe.iter().map(String::as_str).collect();
    let mut used = HashSet::new();
    let mut warnings = Vec::new();
    let mut warned: HashSet<String> = HashSet::new();

    let files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).unwrap_or(e.path());
            if set.is_match(rel) {
                Some(e.into_path())
            } else {
                None
            }
        })
        .collect();

    for path in files {
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        for m in ac.find_overlapping_iter(&contents) {
            used.insert(universe[m.pattern()].clone());
        }
        for token in classlike_tokens(&contents) {
            if !universe_set.contains(token) && looks_like_ours(token) && warned.insert(token.to_string()) {
                let mut d = Diagnostic::error(format!(
                    "'{token}' in {} looks like a PresetCSS class but is not generated",
                    path.display()
                ));
                if let Some(s) = did_you_mean(token, universe.iter().map(String::as_str)) {
                    d = d.with_help(format!("did you mean '{s}'?"));
                }
                warnings.push(d);
            }
        }
    }

    Ok(ScanResult { used, warnings })
}

fn classlike_tokens(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.' | '/')))
        .filter(|t| !t.is_empty())
}

fn looks_like_ours(token: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "bg-", "text-", "border-", "p-", "px-", "py-", "pt-", "pr-", "pb-", "pl-", "m-", "mx-",
        "my-", "mt-", "mr-", "mb-", "ml-", "gap-", "w-", "h-", "max-w-", "min-h-", "font-",
        "rounded-", "shadow-",
    ];
    let stripped = token.rsplit(':').next().unwrap_or(token);
    PREFIXES.iter().any(|p| stripped.starts_with(p))
}
