use std::fmt;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub line: Option<usize>,
    pub file: Option<String>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Diagnostic { message: message.into(), line: None, file: None, help: None }
    }

    pub fn at(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn in_file(mut self, file: &str) -> Self {
        if self.line.is_some() {
            self.file = Some(file.to_string());
        }
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "{}:{l}: {}", self.file.as_deref().unwrap_or("preset.toml"), self.message)?,
            None => write!(f, "{}", self.message)?,
        }
        if let Some(help) = &self.help {
            write!(f, "\n  help: {help}")?;
        }
        Ok(())
    }
}

pub type DiagResult<T> = Result<T, Vec<Diagnostic>>;

pub fn locate(src: &str, section: &str, key: &str) -> Option<usize> {
    let header = format!("[{section}]");
    let mut in_section = section.is_empty();
    for (i, raw) in src.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_section = line == header;
            continue;
        }
        if in_section {
            let lhs = line.split('=').next().unwrap_or("").trim();
            let lhs = lhs.trim_matches('"');
            if lhs == key {
                return Some(i + 1);
            }
        }
    }
    None
}

pub fn did_you_mean<'a>(input: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut best: Option<(f64, &str)> = None;
    for c in candidates {
        let score = strsim::jaro_winkler(input, c);
        if best.is_none_or(|(b, _)| score > b) {
            best = Some((score, c));
        }
    }
    best.filter(|(score, _)| *score > 0.8).map(|(_, c)| c.to_string())
}
