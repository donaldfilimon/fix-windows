mod categories;
mod cli;
mod inventory;
mod layout;
mod scanner;

use anyhow::{Context, Result, ensure};
use clap::{Parser, ValueEnum};
use cli::{Action, Audit, Cli, Paths};
use layout::Layout;
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
    process::ExitCode,
};

struct Report(BufWriter<File>);

impl Report {
    fn new(file: File) -> Self {
        Self(BufWriter::new(file))
    }

    fn emit(&mut self, event: Value) -> Result<()> {
        serde_json::to_writer(&mut self.0, &event)?;
        self.0.write_all(b"\n")?;
        self.0.flush()?;
        Ok(())
    }

    fn sync(&self) -> Result<()> {
        self.0.get_ref().sync_all()?;
        Ok(())
    }
}

fn checked(paths: &Paths) -> Result<Layout> {
    Layout::check(
        &paths.windows_mount,
        &paths.report_directory,
        paths.clam_db.as_deref(),
    )
}
fn run_audit(args: Audit) -> Result<u8> {
    let layout = checked(&args.paths)?;
    let path = layout.output.join(&args.report_name);
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&path)
        .with_context(|| format!("create new report {path:?}"))?;
    // Some removable filesystems ignore creation modes. Never send inventory
    // or scanner data to a report that is readable by other users.
    ensure!(
        file.metadata()?.permissions().mode() & 0o077 == 0,
        "report filesystem did not enforce private permissions; empty file left at {path:?}"
    );
    let mut report = Report::new(file);
    let selection = args.selection;
    let audit = (|| -> Result<u8> {
        report.emit(json!({
            "event": "audit_started",
            "started_utc": chrono::Utc::now().to_rfc3339(),
            "mount": layout.root.to_string_lossy(),
            "content_scan": layout.database.is_some(),
            "inventory": "path metadata only; registry and persistence contents are unparsed",
            "filters": {
                "category": selection.category,
                "surface": selection.surface.and_then(|s| s.to_possible_value()).map(|v| v.get_name().to_owned()),
                "kind": selection.kind.and_then(|k| k.to_possible_value()).map(|v| v.get_name().to_owned()),
                "summary_only": selection.summary_only
            }
        }))?;
        let complete = inventory::inventory(&layout.root, &selection, &mut report)?;
        let scan = if let Some(db) = &layout.database {
            scanner::scan(
                Path::new("/usr/bin/clamscan"),
                &layout.root,
                db,
                &mut report,
            )?
        } else {
            report.emit(
                json!({"event":"scan_result","outcome":"not requested; zero content scanned"}),
            )?;
            0
        };
        let code = if complete { scan } else { 2 };
        report.emit(json!({"event":"audit_finished","exit_code":code}))?;
        report.sync()?;
        Ok(code)
    })();
    if let Err(e) = &audit {
        let _ = report.emit(json!({"event":"audit_interrupted","detail":format!("{e:#}")}));
    }
    eprintln!("Report: {path:?}");
    audit
}
fn run() -> Result<u8> {
    match Cli::parse().command {
        Action::Categories => {
            for c in categories::CATEGORIES {
                println!("{}\t{}\t{}", c.id, c.surface.as_str(), c.label)
            }
        }
        Action::Check(paths) => {
            checked(&paths)?;
            println!("Layout checks passed. No report created; no content scanned.");
        }
        Action::Audit(args) => return run_audit(args),
    }
    Ok(0)
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("Audit stopped: {e:#}");
            ExitCode::from(2)
        }
    }
}
