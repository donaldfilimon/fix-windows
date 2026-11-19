use crate::{
    Report,
    categories::classify,
    cli::{Kind, Selection},
};
use anyhow::Result;
use serde_json::json;
use std::{collections::BTreeMap, os::unix::fs::MetadataExt, path::Path};
use walkdir::WalkDir;

pub fn inventory(root: &Path, selection: &Selection, report: &mut Report) -> Result<bool> {
    let device = root.metadata()?.dev();
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
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
            let Some(cat) = classify(&display) else {
                continue;
            };
            let kind = if meta.is_file() {
                Kind::File
            } else {
                Kind::Directory
            };
            if selection.surface.is_some_and(|s| s != cat.surface)
                || selection.kind.is_some_and(|k| k != kind)
                || (!selection.category.is_empty()
                    && !selection.category.iter().any(|id| id == cat.id))
            {
                continue;
            }
            *counts.entry(cat.label).or_default() += 1;
            if selection.summary_only {
                continue;
            }
            json!({
                "event": "entry", "path": display, "category": cat.label,
                "kind": if meta.is_file() { "file" } else { "directory" },
                "bytes": meta.is_file().then_some(meta.len()),
                "modified_unix": meta.mtime() as f64 + meta.mtime_nsec() as f64 / 1e9
            })
        };
        if relative.to_str().is_none() {
            row["path_bytes"] = json!(relative.as_os_str().as_encoded_bytes());
        }
        report.emit(row)?;
    }
    report.emit(json!({"event":"inventory_summary","counts":counts}))?;
    Ok(!counts.contains_key("errors"))
}
