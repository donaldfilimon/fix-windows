#!/bin/sh
# Run as root inside a disposable Linux VM with CAP_SYS_ADMIN.
set -eu

binary=$(realpath "${1:-$(dirname "$0")/../target/debug/fix-windows}")
test -x "$binary"
fixture=$(mktemp -d)
trap 'umount "$fixture/windows/Users" 2>/dev/null || true
      umount "$fixture/windows" 2>/dev/null || true
      umount "$fixture/reports" 2>/dev/null || true
      if ! mountpoint -q "$fixture/windows" && ! mountpoint -q "$fixture/reports"; then rm -rf "$fixture"; fi' EXIT

chmod 755 "$fixture"
install -m 755 "$binary" "$fixture/fix-windows"
mkdir -p "$fixture/source/Users/Alice/Downloads" "$fixture/source/Windows/System32/Tasks"
mkdir "$fixture/windows" "$fixture/reports"
printf 'personal data\n' > "$fixture/source/Users/Alice/Downloads/example.txt"
touch "$fixture/source/Windows/System32/Tasks/Example Task"
ln -s /etc "$fixture/source/Users/Alice/Downloads/outside"
mkfifo "$fixture/source/Users/Alice/Downloads/pipe"
mount --bind "$fixture/source" "$fixture/windows"
mount -t tmpfs -o mode=1777 tmpfs "$fixture/reports"

audit() {
    runuser -u nobody -- "$fixture/fix-windows" audit "$fixture/windows" "$@"
}

refused() {
    status=0
    audit "$@" || status=$?
    test "$status" -eq 2
}

refused "$fixture/reports"
test ! -e "$fixture/reports/windows-audit.jsonl"
mount -o remount,bind,ro "$fixture/windows"
mkdir -m 777 "$fixture/same-device"
refused "$fixture/same-device"
mount --bind "$fixture/source/Users" "$fixture/windows/Users"
refused "$fixture/reports"
test ! -e "$fixture/reports/windows-audit.jsonl"
umount "$fixture/windows/Users"

runuser -u nobody -- "$fixture/fix-windows" check "$fixture/windows" "$fixture/reports"
test ! -e "$fixture/reports/windows-audit.jsonl"
refused "$fixture/reports" --clam-db "$fixture/windows/Users"
test ! -e "$fixture/reports/windows-audit.jsonl"
audit "$fixture/reports"
cp "$fixture/reports/windows-audit.jsonl" "$fixture/first.jsonl"
refused "$fixture/reports"
cmp "$fixture/first.jsonl" "$fixture/reports/windows-audit.jsonl"
audit "$fixture/reports" --report-name selected.jsonl --category downloads,tasks --surface profile --kind file --summary-only
python3 - "$fixture/reports/selected.jsonl" <<'PY'
import json, sys
rows = [json.loads(line) for line in open(sys.argv[1])]
assert not any(row['event'] == 'entry' for row in rows)
assert next(row['counts'] for row in rows if row['event'] == 'inventory_summary') == {'downloads': 1, 'skipped': 2}
PY
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
if [ -x /usr/bin/clamscan ]; then
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
if [ -n "${CLAM_TEST_DB:-}" ]; then
    /usr/bin/clamscan --database "$CLAM_TEST_DB" --version
    # Use only a scratch-tree EICAR test file, never an executable or live sample.
    audit "$fixture/reports" --report-name clean.jsonl --clam-db "$CLAM_TEST_DB" || {
        cat "$fixture/reports/clean.jsonl" >&2; exit 1;
    }
    printf '%s' 'X5O!P%@AP[4\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*' > "$fixture/source/test.txt"
    status=0
    audit "$fixture/reports" --report-name alert.jsonl --category tasks --clam-db "$CLAM_TEST_DB" || status=$?
    test "$status" -eq 1 || { cat "$fixture/reports/alert.jsonl" >&2; exit 1; }
    python3 - "$fixture/reports/alert.jsonl" <<'PY'
import json, sys
rows = [json.loads(line) for line in open(sys.argv[1])]
assert rows[-1] == {'event': 'audit_finished', 'exit_code': 1}
assert any('test.txt' in r.get('line', '') and 'FOUND' in r['line'] for r in rows)
PY
fi
mkdir -m 000 "$fixture/source/unreadable"
refused "$fixture/reports" --report-name partial.jsonl
python3 - "$fixture/reports/partial.jsonl" <<'PY'
import json, sys
rows = [json.loads(line) for line in open(sys.argv[1])]
assert any(r['event'] == 'inventory_error' for r in rows)
assert rows[-1] == {'event': 'audit_finished', 'exit_code': 2}
PY
echo "Linux fixture passed"
