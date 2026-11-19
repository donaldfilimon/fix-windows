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
# mkdir, not mkdir -p: never overwrite a previously staged package.
mkdir -- "$out"
install -m 755 "$binary" "$out/fix-windows"
set -- README.md PLAN.md Cargo.lock
cp "$@" "$out/"
(
    cd -- "$out"
    printf '%s\n' "$binary_version" "$compiler" "$cargo_version" "$libc" > BUILD.txt
    sha256sum fix-windows "$@" BUILD.txt > SHA256SUMS
    sha256sum --check --strict SHA256SUMS
)
