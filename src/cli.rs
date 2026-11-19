use crate::categories::{CATEGORIES, Surface};
use clap::{Args, Parser, Subcommand, ValueEnum, builder::PossibleValuesParser};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Action,
}
#[derive(Subcommand)]
pub enum Action {
    /// Inventory recovery metadata and optionally run an offline content scan.
    Audit(Audit),
    /// Validate mount, report directory and optional database location; write nothing.
    Check(Paths),
    /// List inventory category IDs, surfaces and report labels.
    Categories,
}
#[derive(Args)]
pub struct Paths {
    /// Exact read-only Windows mountpoint (no nested mounts).
    pub windows_mount: PathBuf,
    /// Existing writable directory on a separate filesystem.
    pub report_directory: PathBuf,
    /// Preloaded official ClamAV database directory outside Windows.
    #[arg(long)]
    pub clam_db: Option<PathBuf>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Kind {
    File,
    Directory,
}
#[derive(Args)]
pub struct Selection {
    /// Only report metadata from this surface.
    #[arg(long, value_enum)]
    pub surface: Option<Surface>,
    /// Include these category IDs (repeatable or comma-separated).
    #[arg(long, value_delimiter=',', value_parser=PossibleValuesParser::new(CATEGORIES.iter().map(|c| c.id)))]
    pub category: Vec<String>,
    /// Only count and report matched entries of this kind.
    #[arg(long, value_enum)]
    pub kind: Option<Kind>,
    /// Omit matched entry rows. Keep counts, skips, errors and scanner output.
    #[arg(long)]
    pub summary_only: bool,
}
#[derive(Args)]
pub struct Audit {
    #[command(flatten)]
    pub paths: Paths,
    #[command(flatten)]
    pub selection: Selection,
    /// New JSONL file in REPORT_DIRECTORY; cannot overwrite an existing file.
    #[arg(long, default_value="windows-audit.jsonl", value_parser=report_name)]
    pub report_name: String,
}
fn report_name(name: &str) -> Result<String, String> {
    if Path::new(name).file_name().and_then(|n| n.to_str()) != Some(name) {
        return Err("expected a single filename, not a path".into());
    }
    Ok(name.into())
}
