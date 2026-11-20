use crate::{
    Report,
    categories::classify,
    cli::{Kind, Selection},
    rescue::{self, Role},
};
use anyhow::Result;
use serde_json::json;
use std::{collections::BTreeMap, os::unix::fs::MetadataExt, path::Path};
use walkdir::WalkDir;

pub fn inventory(root: &Path, selection: &Selection, report: &mut Report) -> Result<bool> {
    let device = root.metadata()?.dev();
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    let mut backup = 0_u64;
    let mut hold = 0_u64;
    let mut persistence = 0_u64;
    let walk = WalkDir::new(root)
        .min_depth(1)
        .follow_links(false)
        .same_file_system(true)
        .into_iter();
    for item in walk {
        let result = item.and_then(|entry| entry.metadata().map(|meta| (entry, meta)));
        let (entry, meta) = match result {
            Ok(pair) => pair,
            Err(e) => {
                *counts.entry("errors").or_default() += 1;
                report.emit(json!({"event":"inventory_error", "detail":e.to_string()}))?;
                continue;
            }
        };
        let relative = entry.path().strip_prefix(root)?;
        let display = relative.to_string_lossy();
        let mut row = if meta.dev() != device || !(meta.is_file() || meta.is_dir()) {
            *counts.entry("skipped").or_default() += 1;
            json!({"event":"skipped_entry", "path":display, "reason":"link, special file, or other filesystem"})
        } else {
            let cat = classify(&display);
            let extra = if cat.is_none() {
                rescue::extra(&display, meta.is_file())
            } else {
                None
            };
            let Some(surface) = cat
                .map(|c| c.surface)
                .or_else(|| extra.map(|item| item.surface))
            else {
                continue;
            };
            let kind = if meta.is_file() {
                Kind::File
            } else {
                Kind::Directory
            };
            if selection.surface.is_some_and(|s| s != surface)
                || selection.kind.is_some_and(|k| k != kind)
                || (!selection.category.is_empty()
                    && !cat.is_some_and(|c| selection.category.iter().any(|id| id == c.id)))
            {
                continue;
            }
            if let Some(cat) = cat {
                *counts.entry(cat.label).or_default() += 1;
            }
            let role = cat
                .and_then(|c| rescue::category_role(c.id, &display, meta.is_file()))
                .or_else(|| extra.map(|item| item.role));
            match role {
                Some(Role::Backup) => backup += 1,
                Some(Role::Hold) => hold += 1,
                Some(Role::Persistence) => persistence += 1,
                None => {}
            }
            if selection.summary_only {
                continue;
            }
            let label = cat
                .map(|c| c.label)
                .unwrap_or_else(|| extra.map(|item| item.label).unwrap_or("personal file"));
            let mut row = json!({
                "event": "entry", "path": display, "category": label,
                "kind": if meta.is_file() { "file" } else { "directory" },
                "bytes": meta.is_file().then_some(meta.len()),
                "modified_unix": meta.mtime() as f64 + meta.mtime_nsec() as f64 / 1e9
            });
            if let Some(role) = role {
                row["role"] = json!(role.as_str());
            }
            row
        };
        if relative.to_str().is_none() {
            row["path_bytes"] = json!(relative.as_os_str().as_encoded_bytes());
        }
        report.emit(row)?;
    }
    report.emit(json!({"event":"inventory_summary","counts":counts}))?;
    report.emit(json!({
        "event": "recovery_plan",
        "backup": backup,
        "hold": hold,
        "persistence": persistence,
        "action": "copy backup files only; review hold, persistence, and scanner alerts; do not delete or repair"
    }))?;
    Ok(!counts.contains_key("errors"))
}
