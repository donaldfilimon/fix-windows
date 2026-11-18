#!/bin/sh
# Disposable Linux VM only. No user-provided volume or device is accepted.
set -eu
[ "$#" = 1 ] || { echo 'Usage: sh tests/ntfs-image.sh AUDITOR' >&2; exit 2; }
binary=$(realpath "$1")
tmp=$(mktemp -d)
cleanup() {
    if mountpoint -q "$tmp/windows"; then umount "$tmp/windows" || return; fi
    if mountpoint -q "$tmp/reports"; then umount "$tmp/reports" || return; fi
    rm -rf -- "$tmp"
}
trap cleanup EXIT
chmod 755 "$tmp"
install -m 755 "$binary" "$tmp/fix-windows"
mkdir "$tmp/windows" "$tmp/reports"
chown nobody "$tmp/reports"
chmod 700 "$tmp/reports"
truncate -s 64M "$tmp/test.ntfs"
mkntfs -F -Q "$tmp/test.ntfs" >/dev/null
ntfs-3g "$tmp/test.ntfs" "$tmp/windows" -o nodev,nosuid,noexec
mkdir -p "$tmp/windows/Users/Alice/Downloads"
printf 'fixture only\n' > "$tmp/windows/Users/Alice/Downloads/example.txt"
umount "$tmp/windows"
sha256sum "$tmp/test.ntfs" > "$tmp/before.sha256"
ntfs-3g "$tmp/test.ntfs" "$tmp/windows" -o "ro,norecover,nodev,nosuid,noexec,allow_other,uid=$(id -u nobody),gid=$(id -g nobody),umask=077"
runuser -u nobody -- "$tmp/fix-windows" check "$tmp/windows" "$tmp/reports"
runuser -u nobody -- "$tmp/fix-windows" audit "$tmp/windows" "$tmp/reports"
python3 - "$tmp/reports/windows-audit.jsonl" <<'PY'
import json, stat, sys
from pathlib import Path
report = Path(sys.argv[1])
assert stat.S_IMODE(report.stat().st_mode) == 0o600
rows = [json.loads(line) for line in report.read_text().splitlines()]
assert any(r.get('path') == 'Users/Alice/Downloads/example.txt' and r.get('category') == 'downloads' and r.get('bytes') == 13 for r in rows)
assert rows[-1] == {'event': 'audit_finished', 'exit_code': 0}
PY
umount "$tmp/windows"
sha256sum -c "$tmp/before.sha256"
echo 'Disposable NTFS image unchanged after audit'

# Reverse the roles: simulate removable report media that ignores mode 0600.
# The app must refuse before writing even the audit_started row.
mkdir "$tmp/empty-source"
mount --bind "$tmp/empty-source" "$tmp/windows"
mount -o remount,bind,ro "$tmp/windows"
ntfs-3g "$tmp/test.ntfs" "$tmp/reports" -o "nodev,nosuid,noexec,allow_other,uid=$(id -u nobody),gid=$(id -g nobody),umask=000"
status=0
runuser -u nobody -- "$tmp/fix-windows" audit "$tmp/windows" "$tmp/reports" || status=$?
test "$status" -eq 2
test -f "$tmp/reports/windows-audit.jsonl"
test ! -s "$tmp/reports/windows-audit.jsonl"
echo 'Non-private report rejected before any audit data was written'
