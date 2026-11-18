#!/bin/sh
# Run as root inside a disposable Linux VM with CAP_SYS_ADMIN.
set -eu

script=$(realpath "$(dirname "$0")/../audit-windows.py")
fixture=$(mktemp -d)
trap 'umount "$fixture/windows/Users" 2>/dev/null || true
      umount "$fixture/windows" 2>/dev/null || true
      umount "$fixture/reports" 2>/dev/null || true
      rm -rf "$fixture"' EXIT

chmod 755 "$fixture"
mkdir -p "$fixture/source/Users/Alice/Downloads" "$fixture/source/Windows/System32/Tasks"
mkdir "$fixture/windows" "$fixture/reports"
printf 'personal data\n' > "$fixture/source/Users/Alice/Downloads/example.txt"
touch "$fixture/source/Windows/System32/Tasks/Example Task"
ln -s /etc "$fixture/source/Users/Alice/Downloads/outside"
mkfifo "$fixture/source/Users/Alice/Downloads/pipe"
mount --bind "$fixture/source" "$fixture/windows"
mount -t tmpfs -o mode=1777 tmpfs "$fixture/reports"

audit() {
    runuser -u nobody -- python3 "$script" "$fixture/windows" "$@"
}

refused() {
    status=0
    audit "$@" || status=$?
    test "$status" -eq 2
}

refused "$fixture/reports"
test ! -e "$fixture/reports/windows-audit.jsonl"
mount -o remount,bind,ro "$fixture/windows"
refused "$fixture/source"
mount --bind "$fixture/source/Users" "$fixture/windows/Users"
refused "$fixture/reports"
test ! -e "$fixture/reports/windows-audit.jsonl"
umount "$fixture/windows/Users"

audit "$fixture/reports"
refused "$fixture/reports"
python3 - "$fixture/reports/windows-audit.jsonl" <<'PY'
import json
import stat
import sys
from pathlib import Path

report = Path(sys.argv[1])
assert stat.S_IMODE(report.stat().st_mode) == 0o600
rows = [json.loads(line) for line in report.read_text().splitlines()]
entries = {row["path"]: row["category"] for row in rows if row["event"] == "entry"}
assert entries["Users/Alice/Downloads/example.txt"] == "downloads"
assert entries["Windows/System32/Tasks/Example Task"] == "scheduled tasks"
assert not any("/outside" in path for path in entries)
skipped = {row["path"] for row in rows if row["event"] == "skipped_entry"}
assert skipped == {"Users/Alice/Downloads/outside", "Users/Alice/Downloads/pipe"}
assert any(row.get("outcome") == "not requested; zero content scanned" for row in rows)
assert rows[-1] == {"event": "audit_finished", "exit_code": 0}
PY
cmp "$fixture/source/Users/Alice/Downloads/example.txt" - <<'DATA'
personal data
DATA
if command -v clamscan >/dev/null; then
    mkdir -m 777 "$fixture/reports/scan"
    mkdir "$fixture/reports/empty-db"
    refused "$fixture/reports/scan" --clam-db "$fixture/reports/empty-db"
    python3 - "$fixture/reports/scan/windows-audit.jsonl" <<'PYTEST'
import json
import sys
from pathlib import Path

rows = [json.loads(line) for line in Path(sys.argv[1]).read_text().splitlines()]
assert any(row["event"] == "scanner_version" for row in rows)
assert rows[-1]["event"] == "audit_interrupted" or rows[-1] == {
    "event": "audit_finished", "exit_code": 2,
}
PYTEST
fi
echo "Linux fixture passed"
