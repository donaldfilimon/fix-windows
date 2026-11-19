#!/bin/sh
# Run from the project root on the trusted GNU/Linux build machine.
set -eu
[ "$#" = 1 ] || { echo 'Usage: sh scripts/package-linux.sh NEW_BUNDLE_DIRECTORY' >&2; exit 2; }
compiler=$(rustc -vV)
host=$(printf '%s\n' "$compiler" | sed -n 's/^host: //p')
case "$host" in x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu) ;; *) echo "Unsupported build host: $host" >&2; exit 2;; esac
case "$1" in /*) out=$1 ;; *) out=$PWD/$1 ;; esac
cargo_version=$(cargo --version)
libc=$(getconf GNU_LIBC_VERSION)
target=${CARGO_TARGET_DIR:-target}
cargo build --locked --release --target-dir "$target" --target "$host"
binary=$target/$host/release/fix-windows
binary_version=$("$binary" --version)
staging=$(mktemp -d -- "$(dirname -- "$out")/.fix-windows.XXXXXXXX")
trap 'rm -rf -- "$staging"' EXIT
trap 'exit 1' HUP INT TERM
bundle=$staging/bundle
mkdir -- "$bundle"
install -m 755 "$binary" "$bundle/fix-windows"
set -- README.md PLAN.md Cargo.lock
cp "$@" "$bundle/"
(
    cd -- "$bundle"
    printf '%s\n' "$binary_version" "$compiler" "$cargo_version" "$libc" > BUILD.txt
    sha256sum fix-windows "$@" BUILD.txt > SHA256SUMS
    sha256sum --check --strict SHA256SUMS
)
# GNU mv -n may report success when it skips an existing destination.
mv --no-target-directory --no-clobber -- "$bundle" "$out"
if [ -d "$bundle" ]; then
    echo "Bundle destination already exists: $out" >&2
    exit 1
fi
