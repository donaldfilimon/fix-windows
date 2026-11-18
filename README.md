# Fix Windows

Use this Linux live-USB workflow to inspect a potentially infected Windows PC while the installed Windows stays off. The Rust 2024 `fix-windows` application inventories file metadata and can run an offline ClamAV scan. The report can guide file recovery and a later clean install.

For development checks, the disposable Linux fixture and release next steps, see [PLAN.md](PLAN.md).

## 1. Prepare on a clean computer

- Have the PC owner approve the boot, inspection, any remote session, and any files to recover or share. For a managed PC, involve its IT administrator. The owner stays present at the PC.
- Download a supported Ubuntu live image for the PC's architecture. Verify the signed checksum list with the trusted Ubuntu signing key, then verify the ISO against that list before writing the USB. Keep Secure Boot enabled and use the firmware's one-time UEFI boot menu. Choose **Try Ubuntu**. If the USB will not boot with Secure Boot enabled, stop and check the image and firmware setup. [Image verification](https://documentation.ubuntu.com/security/software-integrity/image-verification/) · [Secure Boot](https://documentation.ubuntu.com/security/security-features/platform-protections/secure-boot/)
- Prepare a separate external disk for reports and recovered files, plus a trusted Linux build of the application. On a clean Linux machine matching the live system’s architecture and libc, run `sh scripts/package-linux.sh target/offline-kit` and copy the whole new directory to trusted media. On the live system, enter that directory, verify `sha256sum --check --strict SHA256SUMS`, and use `./fix-windows`. Keep a trusted copy of the manifest separately: checksums detect corruption, not replacement of both files and manifest. The live system does not need Cargo or Python to run the audit. Check it for `ntfs-3g` and, if needed, `cryptsetup`. For the optional scan, prepare a compatible `/usr/bin/clamscan` and fresh official signature files on clean media in advance. Stage any needed packages from trusted distro sources. [ClamAV scanning](https://docs.clamav.net/manual/Usage/Scanning.html)
- If BitLocker is enabled, have the owner locate the matching recovery key on a clean device or through IT beforehand. The owner enters it locally at the unlock prompt. Keep it out of chat, command arguments, logs and the report. If the key is unavailable, stop before trying to access the encrypted volume. [Microsoft recovery key guide](https://support.microsoft.com/en-us/windows/security/encryption/find-your-bitlocker-recovery-key)

## 2. Identify and mount Windows read-only

Leave networking disconnected for an offline run. Keep Windows volumes out of the file manager to avoid automounting. From the live terminal, use `lsblk -o PATH,SIZE,MODEL,SERIAL,TRAN,FSTYPE,RO,MOUNTPOINTS` to identify the internal Windows partition and the separate external disk by hardware and size. If any Windows volume was mounted writable, pause and resolve that before proceeding.

The examples use `/dev/nvme0n1p3` as a placeholder. Substitute the owner-confirmed partition. Check the device read-only flag before unlocking or mounting.

```sh
sudo blockdev --setro /dev/nvme0n1p3
sudo blockdev --getro /dev/nvme0n1p3   # must print 1
```

For BitLocker, run the following at the owner's local terminal, then use `/dev/mapper/recovery-windows` as the source in the mount command. Both checks must succeed. [cryptsetup open manual](https://man7.org/linux/man-pages/man8/cryptsetup-open.8.html)

```sh
sudo cryptsetup open --type bitlk --readonly /dev/nvme0n1p3 recovery-windows
sudo blockdev --getro /dev/mapper/recovery-windows   # must print 1
```

For plain NTFS, use the original partition. Use an empty mountpoint. If a command fails, or the mount refuses a dirty or hibernated volume, stop. Preserve hibernation data and skip repair or forced mount options.

```sh
sudo mkdir -p /mnt/windows-ro
sudo mount -t ntfs-3g -o "ro,norecover,nodev,nosuid,noexec,uid=$(id -u),gid=$(id -g),umask=077" \
  /dev/nvme0n1p3 /mnt/windows-ro
findmnt --mountpoint /mnt/windows-ro -o TARGET,SOURCE,FSTYPE,OPTIONS
```

Confirm the source and `ro` in the output and confirm the root contains the expected Windows installation. Leave the mount layout fixed during the audit. Mount the report disk separately and create a new private directory on it for each run. The application checks for a different filesystem. The owner verifies that it is a different physical disk.

## 3. Run the audit

Run from trusted media as the live user, without `sudo`. Replace the output path with the actual existing directory on the external report disk.

```sh
fix-windows check /mnt/windows-ro /media/ubuntu/RECOVERY/audit-01
fix-windows audit /mnt/windows-ro /media/ubuntu/RECOVERY/audit-01
# Optional content scan. Use a fresh output directory.
fix-windows audit /mnt/windows-ro /media/ubuntu/RECOVERY/audit-02 \
  --clam-db /media/ubuntu/RECOVERY/clamav-db
```

Replace the former `python3 audit-windows.py` invocation with `fix-windows audit` (examples assume the trusted binary is on `PATH`). `check` validates the directory layout without writing, scanning, certifying signatures, or reserving a report name. `audit` repeats the checks and creates its report exclusively with mode `0600`. Use storage that enforces Unix permissions.

```sh
fix-windows categories                     # IDs for all eleven categories
fix-windows audit --help                   # all switches and accepted values
fix-windows audit /mnt/windows-ro /media/ubuntu/RECOVERY/audit-01 \
  --category downloads,user-startup --surface profile --kind file \
  --summary-only --report-name profile-counts.jsonl
```

Metadata filters intersect; category IDs within `--category` are alternatives. `--kind` accepts `file` or `directory`; `--surface` accepts `system` or `profile`. `--summary-only` retains category counts, skips, errors and scanner messages, so it is **not** a privacy filter. These options do not limit traversal or narrow a requested ClamAV scan. A partial report after interruption is not a completed audit; keep it and use a new report name for the retry.

`windows-audit.jsonl` records Downloads and startup paths, registry hive locations, scheduled tasks, drivers, group policy and WMI repository metadata. It skips symlinks and special files. Review skipped entries and inventory errors. Custom profile locations, redirected Downloads, other partitions, alternate streams, deleted data, firmware, and running memory are outside this workflow. Registry hives and task contents are not interpreted.

The default run scans zero file contents. With `--clam-db`, review ClamAV's version/signature date, scan summary, errors, alerts and engine limits. Encrypted archives, unreadable data, and size or recursion limits can leave content unexamined. Exit 0 means the requested operations completed with no scanner alerts, 1 means ClamAV reported alerts, and 2 means an error or incomplete operation. Check for an `audit_finished` record. Even a completed scan with no alerts leaves uncertainty about infection. Treat report filenames as private and share only what the owner approves.

## 4. Optional temporary remote help

Use SSH only when the owner approves and both parties already have a trusted private route with access restricted to the helper's device. Keep the public internet, router port forwarding, relay exposure, and persistent remote access out of the setup. Without a trusted route, the owner runs the steps locally and shares the approved report.

In the nonpersistent live session, use a dedicated unprivileged account without sudo privileges. Give that account only approved read access and access to a dedicated report directory. If access cannot be scoped, keep the audit local. Use a one-off foreground OpenSSH daemon with its own configuration and fresh host key stored in live RAM. Bind `ListenAddress` to the private address, restrict that port to the helper's private source address, and verify the actual listener with `ss -ltn`. Set `AllowUsers` to the dedicated account, `PermitRootLogin no`, `AuthenticationMethods publickey`, `PasswordAuthentication no`, `KbdInteractiveAuthentication no`, and `DisableForwarding yes`. Use a dedicated `AuthorizedKeysFile` containing only the helper's temporary public key. Validate the configuration with `sshd -t -f <config>` before running `sshd -D -e -f <config>`. [OpenSSH server configuration](https://man.openbsd.org/sshd_config)

The owner reads the host public key fingerprint locally with `ssh-keygen -lf <host-key.pub>` and confirms it by a separate trusted channel. The helper verifies that fingerprint against the received public key, pins it for the private address and port in a temporary known-hosts file, and connects with `StrictHostKeyChecking=yes` and `UserKnownHostsFile` pointing to that file. Stop on mismatch. At the end, disconnect the helper, stop the dedicated daemon and any remaining session processes, verify the port has closed, remove the temporary authorization and route, and shut down the live session. [OpenSSH client configuration](https://man.openbsd.org/ssh_config)

## 5. Recover files before considering a clean install

With the owner, select personal files from the read-only mount. Copy only the agreed files to separate storage, skipping links, executables, scripts, installers, macro-enabled files and unknown archives. Keep the backup isolated. Compare SHA-256 digests of each selected regular file at source and destination and verify that the owner has the files they need before changing the PC. Scan the backup with updated tools on a clean system before opening or restoring it. Review flagged files and data gaps with the owner.

Close any SSH access. Unmount the Windows volume, close the BitLocker mapping if used, and unmount the recovery disk before shutting down. A clean install is a separate manual decision after backup verification. If approved, use verified official installation media and disconnect the backup disk before installing.
