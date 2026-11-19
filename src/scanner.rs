use crate::Report;
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{
    io::{self, BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
};

const OPTIONS: &[&str] = &[
    "--recursive",
    "--cross-fs=no",
    "--follow-dir-symlinks=0",
    "--follow-file-symlinks=0",
    "--official-db-only=yes",
    "--alert-exceeds-max=yes",
    "--alert-encrypted=yes",
];

pub fn scan(scanner: &Path, root: &Path, database: &Path, report: &mut Report) -> Result<u8> {
    let command = |options: &[&str]| {
        let mut cmd = Command::new(scanner);
        cmd.current_dir("/")
            .stdin(Stdio::null())
            .args(options)
            .arg("--database")
            .arg(database);
        cmd
    };
    let v = command(&[])
        .arg("--version")
        .output()
        .with_context(|| format!("query {scanner:?}; install the distro clamscan package"))?;
    report.emit(json!({
        "event": "scanner_version",
        "exit_code": v.status.code(),
        "detail": String::from_utf8_lossy(&[v.stdout, v.stderr].concat()).trim()
    }))?;
    ensure!(v.status.success(), "unable to query ClamAV version");
    report.emit(json!({
        "event": "scan_started",
        "database": database.to_string_lossy(),
        "options": OPTIONS,
        "scope": "whole supplied mount, subject to engine limits"
    }))?;
    let (reader, writer) = io::pipe()?;
    let mut child = command(OPTIONS)
        .arg("--")
        .arg(root)
        .stdout(writer.try_clone()?)
        .stderr(writer)
        .spawn()
        .context("start ClamAV")?;
    let stream = BufReader::new(reader)
        .split(b'\n')
        .try_for_each(|line| -> Result<()> {
            report.emit(json!({"event":"scanner_output", "line":String::from_utf8_lossy(&line?)}))
        });
    if let Err(error) = stream {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let status = child.wait()?.code();
    let (exit, outcome) = match status {
        Some(0) => (0, "no alerts reported"),
        Some(1) => (1, "alerts reported"),
        _ => (2, "scan error"),
    };
    report.emit(json!({"event":"scan_result", "exit_code":status, "outcome":outcome}))?;
    Ok(exit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    #[test]
    fn engine_status_and_arguments() {
        let tmp = tempfile::tempdir().unwrap();
        let scanner = tmp.path().join("fake scanner");
        for (status, expected) in [(0, 0), (1, 1), (7, 2)] {
            fs::write(&scanner, format!("#!/bin/sh\ncase \"$3\" in --version) echo test-version; exit 0;; esac\nprintf '%s\\n' \"$@\"\necho diagnostic >&2\nexit {status}\n")).unwrap();
            fs::set_permissions(&scanner, fs::Permissions::from_mode(0o700)).unwrap();
            let output = tmp.path().join("report");
            let mut report = Report::new(fs::File::create(&output).unwrap());
            assert_eq!(
                scan(
                    &scanner,
                    Path::new("/test mount"),
                    Path::new("/test db"),
                    &mut report
                )
                .unwrap(),
                expected
            );
            let rows: Vec<serde_json::Value> = fs::read_to_string(output)
                .unwrap()
                .lines()
                .map(|s| serde_json::from_str(s).unwrap())
                .collect();
            let args: Vec<_> = rows.iter().filter_map(|r| r["line"].as_str()).collect();
            assert_eq!(
                args,
                [
                    OPTIONS,
                    &["--database", "/test db", "--", "/test mount", "diagnostic"]
                ]
                .concat()
            );
            assert_eq!(rows.last().unwrap()["exit_code"], status);
        }
    }
}
