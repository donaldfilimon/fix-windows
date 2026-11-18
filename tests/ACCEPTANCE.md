# Recovery audit acceptance record — 2026-11-18

**Scope:** local working tree, not a published release. The packaged executables were built in Debian Bookworm GNU/Linux guests. Fixture installation used networking; the audits ran under `unshare --net` on Ubuntu 24.04. ARM64 ran natively on Apple Silicon; x86-64 userspace ran under Rosetta, not on a physical x86 PC.

| Kit (under `target/`) | Tested executable SHA-256 |
| --- | --- |
| `kit-amd64/fix-windows` | `5d394d6b5f87cc7e33bd42e901921469ab7f04d27631d02f676417db72892cf1` |
| `kit-arm64/fix-windows` | `9f18910e0df00b683e5024ff3e43901a128a75663f0fe17188ecc1c285baa69b` |

On **both** kits, all package manifest entries verified. `tests/linux-fixture.sh` passed with ClamAV 1.5.4 and official daily DB 28184 (15 Nov 2026); cases included a clean fixture, EICAR outside the metadata filter, and an empty database. `tests/ntfs-image.sh` passed: the entire source image digest was unchanged and non-private destination permissions caused rejection with an empty report. The dated DB is a regression input, not a recommendation for current recovery work.

`cargo fmt --all --check`, Clippy with warnings denied, five Rust tests, and actionlint passed locally. No Git remote was configured and no hosted CI run was observed.

**Open acceptance:** boot the approved live image with Secure Boot on the intended physical PC; verify the kit against a separately trusted manifest and repeat the documented workflow on disposable plain NTFS and BitLocker volumes with a separate physical report disk. Compare encrypted source digests before unlock and after close, verify report permissions, and record only results and tool versions, never the recovery key. No physical boot, BitLocker unlock, or customer recovery was performed in this test.
