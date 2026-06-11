use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;

use presetcss_core::emit::{emit_tokens, TokenFormat};
use presetcss_core::generate::{class_universe, generate, STATE_PSEUDOS, STATICS};
use presetcss_core::theme::Group;
use presetcss_core::{build_css, compile_theme, scan, Diagnostic, ResolvedTheme, DEFAULT_PRESET};

#[derive(Parser)]
#[command(name = "presetcss", version, about = "A utility-first CSS framework with a dead-simple preset.")]
#[command(disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(short, long, default_value = "preset.toml")]
        output: PathBuf,
        #[arg(long)]
        force: bool,
    },
    Build {
        #[arg(short, long, default_value = "preset.toml")]
        config: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        minify: bool,
        #[arg(long)]
        verbose: bool,
    },
    Watch {
        #[arg(short, long, default_value = "preset.toml")]
        config: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Validate {
        #[arg(short, long, default_value = "preset.toml")]
        config: PathBuf,
    },
    Explain {
        class: String,
        #[arg(short, long, default_value = "preset.toml")]
        config: PathBuf,
    },
    Export {
        #[arg(long, value_enum, default_value_t = Format::Json)]
        format: Format,
        #[arg(short, long, default_value = "preset.toml")]
        config: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Help {
        #[arg(value_enum)]
        topic: HelpTopic,
        #[arg(short, long, default_value = "preset.toml")]
        config: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Json,
    Js,
    CssVars,
}

#[derive(Clone, Copy, ValueEnum)]
enum HelpTopic {
    Theme,
    Static,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Init { output, force } => cmd_init(&output, force),
        Command::Build { config, output, minify, verbose } => {
            cmd_build(&config, output.as_deref(), minify, verbose).map(|_| ())
        }
        Command::Watch { config, output } => cmd_watch(&config, output),
        Command::Validate { config } => cmd_validate(&config),
        Command::Explain { class, config } => cmd_explain(&class, &config),
        Command::Export { format, config, output } => cmd_export(format, &config, output.as_deref()),
        Command::Help { topic, config } => cmd_help(topic, &config),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(diags) => {
            report(&diags);
            ExitCode::FAILURE
        }
    }
}

fn report(diags: &[Diagnostic]) {
    for d in diags {
        eprintln!("error: {d}");
    }
}

fn load_source(config: &Path) -> Result<(String, PathBuf), Vec<Diagnostic>> {
    if config.exists() {
        let src = std::fs::read_to_string(config)
            .map_err(|e| vec![Diagnostic::error(format!("cannot read {}: {e}", config.display()))])?;
        Ok((src, config.to_path_buf()))
    } else {
        eprintln!("note: {} not found, using built-in defaults", config.display());
        Ok((String::new(), config.to_path_buf()))
    }
}

fn compile(config: &Path) -> Result<(ResolvedTheme, String), Vec<Diagnostic>> {
    let (src, _) = load_source(config)?;
    let label = config.display().to_string();
    match compile_theme(&src) {
        Ok(theme) => Ok((theme, src)),
        Err(diags) => Err(diags.into_iter().map(|d| d.in_file(&label)).collect()),
    }
}

fn cmd_init(output: &Path, force: bool) -> Result<(), Vec<Diagnostic>> {
    if output.exists() && !force {
        return Err(vec![Diagnostic::error(format!(
            "{} already exists (use --force to overwrite)",
            output.display()
        ))]);
    }
    std::fs::write(output, DEFAULT_PRESET)
        .map_err(|e| vec![Diagnostic::error(format!("cannot write {}: {e}", output.display()))])?;
    println!("created {}", output.display());
    Ok(())
}

fn cmd_build(
    config: &Path,
    output_override: Option<&Path>,
    minify_flag: bool,
    verbose: bool,
) -> Result<ResolvedTheme, Vec<Diagnostic>> {
    let (mut theme, _) = compile(config)?;
    if minify_flag {
        theme.options.minify = true;
    }

    let used = if theme.options.content.is_empty() {
        None
    } else {
        let root = config.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let universe = class_universe(&theme);
        let result = scan::scan(root, &theme.options.content, &universe)?;
        if verbose {
            for w in &result.warnings {
                eprintln!("warning: {w}");
            }
        }
        Some(result.used)
    };

    let css = build_css(&theme, used.as_ref());

    let out_path = output_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(&theme.options.output));
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    std::fs::write(&out_path, &css)
        .map_err(|e| vec![Diagnostic::error(format!("cannot write {}: {e}", out_path.display()))])?;

    let n = generate(&theme, used.as_ref()).len();
    println!("built {} ({} rules, {} bytes)", out_path.display(), n, css.len());
    Ok(theme)
}

fn cmd_watch(config: &Path, output: Option<PathBuf>) -> Result<(), Vec<Diagnostic>> {
    if let Err(d) = cmd_build(config, output.as_deref(), false, false) {
        report(&d);
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(150), tx)
        .map_err(|e| vec![Diagnostic::error(format!("watcher: {e}"))])?;

    let root = config.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    debouncer
        .watcher()
        .watch(root, RecursiveMode::Recursive)
        .map_err(|e| vec![Diagnostic::error(format!("watch {}: {e}", root.display()))])?;

    println!("watching {} for changes (ctrl-c to stop)", root.display());
    for event in rx {
        if event.is_ok() {
            match cmd_build(config, output.as_deref(), false, false) {
                Ok(_) => {}
                Err(d) => report(&d),
            }
        }
    }
    Ok(())
}

fn cmd_validate(config: &Path) -> Result<(), Vec<Diagnostic>> {
    compile(config)?;
    println!("{}: ok", config.display());
    Ok(())
}

fn cmd_explain(class: &str, config: &Path) -> Result<(), Vec<Diagnostic>> {
    let (theme, _) = compile(config)?;
    let rules = generate(&theme, None);
    let matches: Vec<_> = rules.iter().filter(|r| r.class == class).collect();
    if matches.is_empty() {
        return Err(vec![Diagnostic::error(format!("no class '{class}' is generated by this preset"))
            .with_help("run `presetcss help theme` to see available token keys")]);
    }
    for r in matches {
        println!("{} {{", r.selector());
        for d in &r.decls {
            println!("  {}: {};", d.prop, d.value);
        }
        println!("}}");
        let media = r.media.as_deref().map(|m| format!(" @media (min-width:{m})")).unwrap_or_default();
        println!("  from {}{}\n", r.src, media);
    }
    Ok(())
}

fn cmd_export(format: Format, config: &Path, output: Option<&Path>) -> Result<(), Vec<Diagnostic>> {
    let (theme, _) = compile(config)?;
    let fmt = match format {
        Format::Json => TokenFormat::Json,
        Format::Js => TokenFormat::Js,
        Format::CssVars => TokenFormat::CssVars,
    };
    let out = emit_tokens(&theme, fmt);
    match output {
        Some(path) => {
            std::fs::write(path, &out)
                .map_err(|e| vec![Diagnostic::error(format!("cannot write {}: {e}", path.display()))])?;
            println!("wrote {}", path.display());
        }
        None => print!("{out}"),
    }
    Ok(())
}

fn cmd_help(topic: HelpTopic, config: &Path) -> Result<(), Vec<Diagnostic>> {
    let (theme, _) = compile(config)?;
    match topic {
        HelpTopic::Theme => {
            println!("Token groups and keys:\n");
            for g in Group::ALL {
                let map = theme.group(g);
                let keys: Vec<&str> = map.keys().map(String::as_str).collect();
                println!("[{}] ({} keys)\n  {}\n", g.key(), keys.len(), keys.join(", "));
            }
            let bps: Vec<String> = theme
                .breakpoints
                .iter()
                .map(|(k, v)| format!("{k} ({v})"))
                .collect();
            println!("[breakpoints]\n  {}\n", bps.join(", "));
            println!("States: {}", STATE_PSEUDOS.iter().map(|(s, _)| *s).collect::<Vec<_>>().join(", "));
        }
        HelpTopic::Static => {
            println!("Built-in static utilities ({}):\n", STATICS.len());
            for (class, _) in STATICS {
                println!("  {class}");
            }
        }
    }
    Ok(())
}
