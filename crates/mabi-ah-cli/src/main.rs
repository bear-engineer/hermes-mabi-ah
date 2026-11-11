//! mabi-ah: Mabinogi auction house CLI (composition root).
mod commands;
mod config;
mod context;
mod exit;
mod output;
mod repo_data;

use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind as ClapKind;
use clap::{Parser, ValueEnum};
use mabi_ah_app::error::AppError;
use mabi_ah_nexon::limits::Tier;

use commands::Command;
use commands::watch::{Emit, WatchCommand};
use config::{Config, Format};
use context::{Context, build_meta};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Json,
    Table,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TierArg {
    Development,
    Service,
}

#[derive(Debug, Parser)]
#[command(
    name = "mabi-ah",
    version,
    about = "Mabinogi auction house CLI (NEXON Open API)"
)]
pub(crate) struct Cli {
    /// Output format.
    #[arg(long, value_enum, default_value = "json", global = true)]
    format: FormatArg,
    /// SQLite database path (default: the user data dir).
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    /// File holding the NEXON Open API key.
    #[arg(long, global = true)]
    key_file: Option<PathBuf>,
    /// API host override for tests (env MABI_AH_BASE_URL).
    #[arg(long, hide = true, global = true)]
    base_url: Option<String>,
    /// API quota tier.
    #[arg(long, value_enum, default_value = "development", global = true)]
    tier: TierArg,
    /// Use the whole daily quota instead of 90% of it.
    #[arg(long, global = true)]
    force_budget: bool,
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    fn config(&self) -> Config {
        Config {
            format: match self.format {
                FormatArg::Json => Format::Json,
                FormatArg::Table => Format::Table,
            },
            db_path: self
                .db
                .clone()
                .unwrap_or_else(|| mabi_ah_store::data_dir().join("mabi-ah.db")),
            key_file: self.key_file.clone(),
            base_url: self
                .base_url
                .clone()
                .or_else(|| std::env::var("MABI_AH_BASE_URL").ok())
                .filter(|s| !s.is_empty()),
            tier: match self.tier {
                TierArg::Development => Tier::Development,
                TierArg::Service => Tier::Service,
            },
            force_budget: self.force_budget,
        }
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => return usage_failure(&e),
    };
    let cfg = cli.config();
    let format = cfg.format;
    let name = cli.command.name();
    let raw = is_raw(&cli.command);
    let (envelope, code) = match Context::open(cfg) {
        Err(e) => (output::failure(name, &e, build_meta(None, 0)), exit::of(&e)),
        Ok(mut ctx) => {
            let before = ctx.used_today().unwrap_or(0);
            match cli.command.run(&mut ctx) {
                Ok(data) => (output::ok(name, data, ctx.meta(before)), ExitCode::SUCCESS),
                Err(e) => (output::failure(name, &e, ctx.meta(before)), exit::of(&e)),
            }
        }
    };
    if let Some(err) = &envelope.error {
        eprintln!("mabi-ah: {} [{}]", err.message_ko, err.code);
        if raw {
            // Raw paths (cron) must keep stdout empty on failure.
            return code;
        }
    }
    emit(&output::render(format, &envelope), code)
}

/// Commands whose success output bypasses the envelope (`collect --quiet`,
/// `watch check --emit text`); their failures go to stderr only.
fn is_raw(command: &Command) -> bool {
    match command {
        Command::Collect(a) => a.quiet,
        Command::Watch(w) => matches!(
            &w.command,
            WatchCommand::Check(c) if matches!(c.emit, Emit::Text)
        ),
        _ => false,
    }
}

/// clap rejected the command line: help/version go to stdout, anything else is
/// a usage envelope (exit 2) with the clap text on stderr.
fn usage_failure(e: &clap::Error) -> ExitCode {
    if matches!(e.kind(), ClapKind::DisplayHelp | ClapKind::DisplayVersion) {
        return emit(e.to_string().trim_end(), ExitCode::SUCCESS);
    }
    let err = AppError::Usage(e.to_string().lines().next().unwrap_or_default().to_owned());
    let envelope = output::failure("", &err, build_meta(None, 0));
    eprintln!("{}", e.to_string().trim_end());
    emit(&output::render(Format::Json, &envelope), exit::of(&err))
}

/// Writes one line to stdout; a closed pipe is a clean exit.
fn emit(text: &str, code: ExitCode) -> ExitCode {
    let mut out = std::io::stdout().lock();
    match writeln!(out, "{text}").and_then(|()| out.flush()) {
        Ok(()) => code,
        Err(e) if e.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(exit::INTERNAL),
    }
}
