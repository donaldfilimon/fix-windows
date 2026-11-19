#!/bin/sh
# Exercise the GNU/Linux packager without compiling or mounting anything.
set -eu
unset CDPATH
packager=$(dirname "$0")/../scripts/package-linux.sh
packager=$(cd -- "$(dirname "$packager")" && pwd)/package-linux.sh
fixture=$(mktemp -d)
trap 'rm -rf -- "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/elsewhere/bundle" "$fixture/project"
cd -- "$fixture/project"
touch README.md PLAN.md Cargo.lock

cat > "$fixture/bin/rustc" <<'SH'
#!/bin/sh
printf '%s\n' 'rustc fixture' 'host: aarch64-unknown-linux-gnu'
SH
cat > "$fixture/bin/cargo" <<'SH'
#!/bin/sh
if [ "$1" = --version ]; then
    test "${FAIL_CARGO_VERSION:-0}" -eq 0 || exit 42
    printf '%s\n' 'cargo fixture'
    exit
fi
set -eu
test "$#" -eq 7
test "$5" = 'build output'
test "$7" = aarch64-unknown-linux-gnu
binary="$5/$7/release/fix-windows"
mkdir -p "$(dirname "$binary")"
printf '%s\n' '#!/bin/sh' 'echo "fix-windows fixture"' > "$binary"
chmod +x "$binary"
SH
cat > "$fixture/bin/getconf" <<'SH'
#!/bin/sh
printf '%s\n' 'glibc fixture'
SH
chmod +x "$fixture/bin/"*
export PATH="$fixture/bin:$PATH" CARGO_TARGET_DIR='build output'

CDPATH="$fixture/elsewhere" sh "$packager" bundle
test ! -e "$fixture/elsewhere/bundle/BUILD.txt"
(cd bundle && sha256sum --check --strict SHA256SUMS)
cmp bundle/BUILD.txt - <<'DATA'
fix-windows fixture
rustc fixture
host: aarch64-unknown-linux-gnu
cargo fixture
glibc fixture
DATA

if sh "$packager" bundle; then
    echo 'packager accepted an existing destination' >&2
    exit 1
fi
(cd bundle && sha256sum --check --strict SHA256SUMS)

status=0
FAIL_CARGO_VERSION=1 sh "$packager" incomplete || status=$?
test "$status" -eq 42
test ! -e incomplete
