# Recovery audit acceptance record — 2026-11-19

**Scope:** software close with GitHub remote, not physical live-media acceptance. Kits were built and fixture-tested in disposable Apple `container` GNU/Linux Bookworm guests (`CAP_SYS_ADMIN`). Fixture package installs used networking; audits ran under `unshare --net`. ARM64 ran natively; x86-64 userspace ran under Rosetta, not on a physical x86 PC.

**Source:** report directory fsync on finish/interrupt, `policy-users` (GroupPolicyUsers) inventory, shared-startup order and `Surface::as_str` clap locks. Remote: `https://github.com/donaldfilimon/fix-windows`.

| Kit | Tested executable SHA-256 |
| --- | --- |
| arm64 `bundle/fix-windows` | `6e9410a51053d87dced7bd20021856bf00803d0243e22979f2025ffc8d671805` |
| amd64 `bundle-amd64/fix-windows` | `e017564011a4105f71bdd4a4db272d01acd07c6c050544ca516f8906559c6b01` |

On **both** kits, all package manifest entries verified. `tests/package-linux.sh` passed in the ARM64 guest. `tests/linux-fixture.sh` passed with ClamAV 1.4.6 and CVD signatures from `clamav/clamav:1.4` (`daily.cvd` / `main.cvd` / `bytecode.cvd`, reported as `28188` / 19 Nov 2026); cases included a clean fixture, EICAR outside the metadata filter, and an empty database. `tests/ntfs-image.sh` passed: the entire source image digest was unchanged and non-private destination permissions caused rejection with an empty report. The dated DB is a regression input, not a recommendation for current recovery work.

Host `cargo fmt --all --check`, Clippy with warnings denied, eight Rust tests, and `cargo test --locked` passed on macOS ARM64 (rustc/cargo 1.100.0). Gate evidence: `.work/close-out/` (gitignored).

**Open acceptance:** boot the approved live image with Secure Boot on the intended physical PC; verify the kit against a separately trusted manifest and repeat the documented workflow on disposable plain NTFS and BitLocker volumes with a separate physical report disk. Compare encrypted source digests before unlock and after close, verify report permissions, and record only results and tool versions, never the recovery key. No physical boot, BitLocker unlock, or customer recovery was performed in this test.
