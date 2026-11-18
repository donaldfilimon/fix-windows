use anyhow::{Context, Result, ensure};
use nix::{
    sys::statvfs::{FsFlags, statvfs},
    unistd::{AccessFlags, Uid, access},
};
use std::{
    ffi::OsString,
    fs,
    os::unix::{ffi::OsStringExt, fs::MetadataExt},
    path::{Path, PathBuf},
};

pub struct Layout {
    pub root: PathBuf,
    pub output: PathBuf,
    pub database: Option<PathBuf>,
}

// Linux escapes whitespace and backslashes in mountinfo, even on non-UTF-8 paths.
fn mount_path(raw: &[u8]) -> Result<PathBuf> {
    let mut path = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' {
            let c = match raw.get(i + 1..i + 4) {
                Some(b"040") => b' ',
                Some(b"011") => b'\t',
                Some(b"012") => b'\n',
                Some(b"134") => b'\\',
                _ => anyhow::bail!("unexpected mountinfo escape"),
            };
            path.push(c);
            i += 4;
        } else {
            path.push(raw[i]);
            i += 1;
        }
    }
    Ok(OsString::from_vec(path).into())
}

fn exact_readonly(table: &[u8], root: &Path) -> Result<()> {
    let mut exact = 0;
    for line in table.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        let mut fields = line.split(|b| *b == b' ');
        let path = mount_path(fields.nth(4).context("missing mountinfo path")?)?;
        let flags = fields.next().context("missing mountinfo flags")?;
        if path == root {
            ensure!(
                flags.split(|b| *b == b',').any(|f| f == b"ro"),
                "Windows mount is writable"
            );
            exact += 1;
        } else {
            ensure!(
                !path.starts_with(root),
                "Windows root contains a nested mount"
            );
        }
    }
    ensure!(
        exact == 1,
        "Windows root must be one exact read-only mountpoint"
    );
    Ok(())
}

impl Layout {
    pub fn check(root: &Path, output: &Path, database: Option<&Path>) -> Result<Self> {
        ensure!(
            cfg!(target_os = "linux") && !Uid::effective().is_root(),
            "run as an unprivileged user in a Linux live session"
        );
        let dir = |p: &Path| -> Result<PathBuf> {
            let resolved = p.canonicalize().with_context(|| format!("resolve {p:?}"))?;
            ensure!(resolved.metadata()?.is_dir(), "not a directory: {p:?}");
            Ok(resolved)
        };
        let root = dir(root)?;
        exact_readonly(
            &fs::read("/proc/self/mountinfo").context("read mountinfo")?,
            &root,
        )?;
        ensure!(
            statvfs(root.as_path())?
                .flags()
                .contains(FsFlags::ST_RDONLY),
            "Windows filesystem is writable"
        );
        let device = root.metadata()?.dev();
        let external = |p: &Path| -> Result<PathBuf> {
            let p = dir(p)?;
            ensure!(
                !p.starts_with(&root) && p.metadata()?.dev() != device,
                "directory must be outside Windows on a separate filesystem: {p:?}"
            );
            Ok(p)
        };
        let output = external(output)?;
        ensure!(
            !statvfs(output.as_path())?
                .flags()
                .contains(FsFlags::ST_RDONLY),
            "report filesystem is read-only"
        );
        access(output.as_path(), AccessFlags::W_OK | AccessFlags::X_OK)
            .context("report directory is not writable/searchable")?;
        let database = database.map(external).transpose()?;
        Ok(Self {
            root,
            output,
            database,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mount_table_guards() {
        let row = b"1 2 0:1 / /mnt/win\\040disk ro,nodev - tmpfs tmpfs rw\n";
        let root = Path::new("/mnt/win disk");
        assert!(exact_readonly(row, root).is_ok());
        assert!(exact_readonly(row, &root.join("Users")).is_err());
        assert!(exact_readonly(&[&row[..], row].concat(), root).is_err());
        assert!(exact_readonly(row, Path::new("/mnt")).is_err());
        assert!(exact_readonly(b"1 2 0:1 / /mnt/win\\040disk rw - tmpfs tmpfs ro", root).is_err());
    }
}
