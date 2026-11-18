# Completion and next steps

The migration replaces the Python runtime, not the recovery operator. Disk identification, BitLocker unlock, mounting, file recovery and reinstallation remain manual. There is no force, quarantine, or repair mode.

## Implemented

- One Rust 2024 binary: `check` (non-writing layout validation), `categories` (discovery), `audit` (metadata inventory with optional full-mount ClamAV).
- Keep metadata selection independent from content scanning. Preserve JSONL events, non-overwriting reports, and exit codes; record filter choices in the report.
- Exercise mount and scanner behavior with disposable data; never point the privileged fixture at a Windows installation.

## Repeatable checks

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
```

In a disposable Linux VM with `CAP_SYS_ADMIN`, util-linux (`runuser`, `mount`), Python 3 (test assertions only) and `/usr/bin/clamscan`, run `sudo sh tests/linux-fixture.sh target/release/fix-windows`. Only the setup uses root; the auditor runs as `nobody`. To also test a clean scan and an EICAR alert, pass `CLAM_TEST_DB` to the fixture as an absolute path to official signatures on a separate filesystem readable by `nobody`. Without a scanner the metadata fixture still runs; it is not evidence of scanner qualification.

## Release procedure and acceptance criteria

1. **Live-media acceptance.** Boot the intended Ubuntu live image on the target architecture. Run `check` and `audit` against disposable NTFS and BitLocker test volumes with a separate report disk. Accept when source digests are unchanged, reports are private, the key never enters logs, and unsafe layouts fail before report creation.
2. **Stage and verify.** On GNU/Linux x86_64 or aarch64, run `sh scripts/package-linux.sh target/offline-kit` from the repo root. The new directory contains the binary, docs, dependency lockfile, `BUILD.txt` (compiler, target and build-host libc) and `SHA256SUMS`. Scanner, DB and OS packages are separate prerequisites, not part of this kit. Record `/usr/bin/clamscan --database /path/to/official-db --version` with the binary checksum on the actual live OS. Verify checksums and repeat clean/alert/error cases disconnected before recovery.
3. **Regression automation.** `.github/workflows/linux.yml` defines fmt, lint, tests, native x86_64/arm64 package builds and offline staged-binary fixtures on disposable hosted Ubuntu runners. It requires clean/alert/error scanner cases; the image DB is for regression only, not freshness qualification. `sudo sh tests/ntfs-image.sh target/offline-kit/fix-windows` creates a temporary NTFS image, refuses external device arguments, and compares the entire image digest across the read-only audit. Never run privileged fixtures on a recovery machine. A green CI run is not a physical live-USB or BitLocker acceptance receipt.
