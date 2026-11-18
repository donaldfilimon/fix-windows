#!/usr/bin/env python3
"""Inventory Windows recovery surfaces from a read-only Linux mount."""
import argparse
import collections
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys

SYSTEM_PATHS = {
    "windows/system32/config": "system registry",
    "windows/system32/tasks": "scheduled tasks",
    "windows/system32/drivers": "drivers",
    "windows/system32/wbem/repository": "WMI repository",
    "windows/system32/grouppolicy": "local group policy",
    "programdata/microsoft/windows/start menu/programs/startup": "shared startup",
    "programdata/microsoft/windows/start menu/programs": "shared Start menu",
}
PROFILE_PATHS = {
    "downloads": "downloads",
    "appdata/roaming/microsoft/windows/start menu/programs/startup": "user startup",
}


def mountpoints():
    with open("/proc/self/mountinfo", encoding="utf-8", errors="surrogateescape") as table:
        for line in table:
            fields = line.split(" - ", 1)[0].split()
            path = re.sub(r"\\([0-7]{3})", lambda m: chr(int(m[1], 8)), fields[4])
            yield Path(path), set(fields[5].split(","))


def check_paths(root, output):
    mounts = list(mountpoints())
    root_mounts = [flags for path, flags in mounts if path == root]
    if (len(root_mounts) != 1 or "ro" not in root_mounts[0]
            or not os.statvfs(root).f_flag & os.ST_RDONLY):
        raise ValueError("Windows root must be an exact read-only mountpoint")
    if any(root in path.parents for path, _ in mounts):
        raise ValueError("Windows root contains nested mounts")
    if root == output or root in output.parents or root.stat().st_dev == output.stat().st_dev:
        raise ValueError("report directory must be on a separate filesystem outside Windows")
    if not output.is_dir() or os.statvfs(output).f_flag & os.ST_RDONLY:
        raise ValueError("report destination must be a writable directory")


def category(relative):
    parts = relative.lower().split("/")
    if len(parts) >= 3 and parts[0] == "users":
        if parts[2] == "ntuser.dat":
            return "user registry"
        if "/".join(parts[2:]) == "appdata/local/microsoft/windows/usrclass.dat":
            return "user classes registry"
        for prefix, label in PROFILE_PATHS.items():
            tail = "/".join(parts[2:])
            if tail == prefix or tail.startswith(prefix + "/"):
                return label
    for prefix, label in SYSTEM_PATHS.items():
        if relative.lower() == prefix or relative.lower().startswith(prefix + "/"):
            return label
    return None


def inventory(root, emit):
    counts = collections.Counter()
    device = root.stat().st_dev

    def error(exc):
        counts["errors"] += 1
        emit("inventory_error", detail=str(exc))

    for directory, dirs, files in os.walk(root, onerror=error, followlinks=False):
        for name in dirs + files:
            path = Path(directory) / name
            relative = path.relative_to(root).as_posix()
            try:
                info = path.lstat()
            except OSError as exc:
                error(exc)
                if name in dirs:
                    dirs.remove(name)
                continue
            is_dir = stat.S_ISDIR(info.st_mode)
            is_file = stat.S_ISREG(info.st_mode)
            if info.st_dev != device or not (is_dir or is_file):
                counts["skipped"] += 1
                emit("skipped_entry", path=relative, reason="link, special file, or other filesystem")
                if name in dirs:
                    dirs.remove(name)
                continue
            label = category(relative)
            if label:
                counts[label] += 1
                emit("entry", path=relative, category=label, kind="directory" if is_dir else "file",
                     bytes=info.st_size if is_file else None, modified_unix=info.st_mtime)
    emit("inventory_summary", counts=dict(counts))
    return counts["errors"] == 0


def scan(root, database, emit):
    scanner = shutil.which("clamscan")
    if scanner is None:
        raise ValueError("clamscan is unavailable in the live environment")
    version = subprocess.run(
        [scanner, "--database", str(database), "--version"], capture_output=True,
        text=True, errors="replace", cwd="/", stdin=subprocess.DEVNULL,
    )
    emit("scanner_version", exit_code=version.returncode,
         detail=(version.stdout + version.stderr).strip())
    if version.returncode:
        raise ValueError("unable to query ClamAV version")
    command = [scanner, "--recursive", "--cross-fs=no", "--follow-dir-symlinks=0",
               "--follow-file-symlinks=0", "--official-db-only=yes", "--alert-exceeds-max=yes",
               "--alert-encrypted=yes", "--database", str(database)]
    emit("scan_started", database=str(database), options=command[1:-2],
         scope="ClamAV recursive scan of the supplied mount, subject to engine limits")
    with subprocess.Popen(command + ["--", str(root)], stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, text=True, errors="replace",
                          cwd="/", stdin=subprocess.DEVNULL) as process:
        for line in process.stdout:
            emit("scanner_output", line=line.rstrip("\n"))
        status = process.wait()
    outcome = {0: "no alerts reported", 1: "alerts reported"}.get(status, "scan error")
    emit("scan_result", exit_code=status, outcome=outcome)
    return status if status in (0, 1) else 2


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("windows_mount", type=Path)
    parser.add_argument("report_directory", type=Path)
    parser.add_argument("--clam-db", type=Path, help="preloaded official ClamAV database directory")
    args = parser.parse_args()
    try:
        if sys.platform != "linux" or os.geteuid() == 0:
            raise ValueError("run as an unprivileged user in the Linux live session")
        root = args.windows_mount.resolve(strict=True)
        output = args.report_directory.resolve(strict=True)
        check_paths(root, output)
        database = args.clam_db.resolve(strict=True) if args.clam_db else None
        if database and (not database.is_dir() or database == root or root in database.parents
                         or database.stat().st_dev == root.stat().st_dev):
            raise ValueError("signature database must be a trusted directory outside Windows")
        report = output / "windows-audit.jsonl"
        fd = os.open(report, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8", buffering=1) as stream:
            def emit(event, **fields):
                print(json.dumps({"event": event, **fields}), file=stream)

            try:
                emit("audit_started", mount=str(root), content_scan=bool(database),
                     started_utc=datetime.now(timezone.utc).isoformat(),
                     inventory="path metadata only; registry and persistence contents are unparsed")
                inventory_ok = inventory(root, emit)
                status = scan(root, database, emit) if database else 0
                if not database:
                    emit("scan_result", outcome="not requested; zero content scanned")
                status = status if inventory_ok else 2
                emit("audit_finished", exit_code=status)
            except (OSError, ValueError, KeyboardInterrupt) as exc:
                emit("audit_interrupted", detail=str(exc) or type(exc).__name__)
                raise
        print(f"Report saved to {str(report)!r}")
        return status
    except (OSError, ValueError, KeyboardInterrupt) as exc:
        print(f"Audit stopped: {str(exc)!r}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
