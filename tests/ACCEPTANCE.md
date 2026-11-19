# Recovery audit acceptance record — 2026-11-19

**Scope:** local software close of the working tree, not a published release and not physical live-media acceptance. Kits were built and fixture-tested in disposable Apple `container` GNU/Linux Bookworm guests (`CAP_SYS_ADMIN`). Fixture package installs used networking; audits ran under `unshare --net`. ARM64 ran natively; x86-64 userspace ran under Rosetta, not on a physical x86 PC.

**Source:** tree after `0e0c4e5` (Report emit/sync API) plus the local-close edits in this commit (`.gitignore` for `.work/`, `Surface::as_str` for `categories` output).

| Kit | Tested executable SHA-256 |
| --- | --- |
| arm64 `bundle/fix-windows` | `03c95efae7d5440a50a8e3a6bbc5c98fefe1f60b4430607d631c65d6f160d61b` |
| amd64 `bundle-amd64/fix-windows` | `57bea08d6ea31c42f7d9ad80cf90800d603fe8f6af86b24ec26006258cf7e5e2` |

On **both** kits, all package manifest entries verified. `tests/package-linux.sh` passed in the ARM64 guest. `tests/linux-fixture.sh` passed with ClamAV 1.4.6 and CVD signatures from `clamav/clamav:1.4` (`daily.cvd` / `main.cvd` / `bytecode.cvd`, reported as `28188` / 19 Nov 2026); cases included a clean fixture, EICAR outside the metadata filter, and an empty database. `tests/ntfs-image.sh` passed: the entire source image digest was unchanged and non-private destination permissions caused rejection with an empty report. The dated DB is a regression input, not a recommendation for current recovery work.

Host `cargo fmt --all --check`, Clippy with warnings denied, five Rust tests, and `cargo build --locked --release` passed on macOS ARM64 (rustc/cargo 1.100.0). Gate evidence: `.work/close-out/` (gitignored). No Git remote was configured and no hosted CI run was observed.

**Open acceptance:** create a git remote and observe `.github/workflows/linux.yml`; boot the approved live image with Secure Boot on the intended physical PC; verify the kit against a separately trusted manifest and repeat the documented workflow on disposable plain NTFS and BitLocker volumes with a separate physical report disk. Compare encrypted source digests before unlock and after close, verify report permissions, and record only results and tool versions, never the recovery key. No physical boot, BitLocker unlock, or customer recovery was performed in this test.
