#!/bin/sh
# Run from the project root on the trusted GNU/Linux build machine.
set -eu
[ "$#" = 1 ] || { echo 'Usage: sh scripts/package-linux.sh NEW_BUNDLE_DIRECTORY' >&2; exit 2; }
host=$(rustc -vV | sed -n 's/^host: //p')
case "$host" in x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu) ;; *) echo "Unsupported build host: $host" >&2; exit 2;; esac
out=$1
target=${CARGO_TARGET_DIR:-target}
cargo build --locked --release --target-dir "$target" --target "$host"
# mkdir, not mkdir -p: never overwrite a previously staged package.
mkdir -- "$out"
install -m 755 "$target/$host/release/fix-windows" "$out/fix-windows"
set -- README.md PLAN.md Cargo.lock
cp "$@" "$out/"
(
    cd -- "$out"
    { ./fix-windows --version; rustc -vV; cargo --version; getconf GNU_LIBC_VERSION; } > BUILD.txt
    sha256sum fix-windows "$@" BUILD.txt > SHA256SUMS
    sha256sum --check --strict SHA256SUMS
)
