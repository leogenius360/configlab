#!/bin/sh
set -eu

ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

VERSION=$(awk -F '"' '/^version = "/ { print $2; exit }' Cargo.toml)
if [ -z "$VERSION" ]; then
  echo "package check error: could not determine package version" >&2
  exit 1
fi

PACKAGE_TMP_ROOT="$ROOT/target/package-check"
mkdir -p "$PACKAGE_TMP_ROOT"
TEMP_DIR=$(mktemp -d "$PACKAGE_TMP_ROOT/run.XXXXXX")
trap 'rm -rf "$TEMP_DIR"' EXIT HUP INT TERM

# The proc-macro companion can be verified directly because all of its
# dependencies are registry packages. The main package is assembled without
# Cargo's built-in verification because its exact companion version has not yet
# been published; the independent consumer below verifies both extracted
# packages together through a local registry patch.
cargo package --locked --offline --allow-dirty -p configlab-macros
cargo package --locked --offline --allow-dirty -p configlab --no-verify --exclude-lockfile

MACRO_CRATE="target/package/configlab-macros-$VERSION.crate"
MAIN_CRATE="target/package/configlab-$VERSION.crate"
[ -f "$MACRO_CRATE" ] || { echo "package check error: missing $MACRO_CRATE" >&2; exit 1; }
[ -f "$MAIN_CRATE" ] || { echo "package check error: missing $MAIN_CRATE" >&2; exit 1; }

MAIN_FILES="$TEMP_DIR/main-package-files"
MACRO_FILES="$TEMP_DIR/macro-package-files"
tar -tzf "$MAIN_CRATE" | sed '/\/$/d' > "$MAIN_FILES"
tar -tzf "$MACRO_CRATE" | sed '/\/$/d' > "$MACRO_FILES"

LEAKED=$(grep -E '/(examples|script|target|macros)/' "$MAIN_FILES" || true)
if [ -n "$LEAKED" ]; then
  echo "package check error: workspace-only files leaked into the main crate package:" >&2
  printf '%s\n' "$LEAKED" | sed 's/^/  /' >&2
  exit 1
fi

# Integration tests are intentionally shipped with the source package. Cargo
# recognizes them as package targets; including them avoids producing a package
# manifest that refers to targets whose source files were omitted.
for test_entry in facade kernel performance_baseline resolution runtime source_api; do
  grep -E "/tests/$test_entry\.rs$" "$MAIN_FILES" >/dev/null || {
    echo "package check error: main package is missing tests/$test_entry.rs" >&2
    exit 1
  }
done

grep -E '/src/lib\.rs$' "$MAIN_FILES" >/dev/null || {
  echo "package check error: main package is missing src/lib.rs" >&2
  exit 1
}
grep -E '/README\.md$' "$MAIN_FILES" >/dev/null || {
  echo "package check error: main package is missing README.md" >&2
  exit 1
}
for release_file in CHANGELOG.md LICENSE; do
  grep -E "/$release_file$" "$MAIN_FILES" >/dev/null || {
    echo "package check error: main package is missing $release_file" >&2
    exit 1
  }
done
for license_file in LICENSE; do
  grep -E "/$license_file$" "$MACRO_FILES" >/dev/null || {
    echo "package check error: macro package is missing $license_file" >&2
    exit 1
  }
done
grep -E '/src/lib\.rs$' "$MACRO_FILES" >/dev/null || {
  echo "package check error: macro package is missing src/lib.rs" >&2
  exit 1
}

mkdir -p "$TEMP_DIR/macro" "$TEMP_DIR/main" "$TEMP_DIR/consumer/src"
tar -xzf "$MACRO_CRATE" -C "$TEMP_DIR/macro"
tar -xzf "$MAIN_CRATE" -C "$TEMP_DIR/main"
cat > "$TEMP_DIR/consumer/Cargo.toml" <<EOF_MANIFEST
[package]
name = "configlab-package-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.97"
publish = false

[dependencies]
renamed-configlab = { package = "configlab", path = "../main/configlab-$VERSION" }

[patch.crates-io]
configlab-macros = { path = "../macro/configlab-macros-$VERSION" }

[workspace]
EOF_MANIFEST

cat > "$TEMP_DIR/consumer/src/main.rs" <<'EOF_RUST'
use renamed_configlab::{args, env, path, Config};

#[derive(Debug, Config)]
struct ConsumerConfig {
    #[config(default = 8080, env = "APP_PORT", cli = "port")]
    port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let missing = std::env::temp_dir().join("configlab-package-consumer-missing.toml");
    let config = ConsumerConfig::builder()
        .file(path(missing).optional())
        .environment(env().from([("APP_PORT", "9090")]))
        .arguments(args().from(["--port", "9191"]))
        .load()?;
    assert_eq!(config.port, 9191);
    Ok(())
}
EOF_RUST

(
  cd "$TEMP_DIR/consumer"
  cargo check --offline
  cargo run --offline --quiet
)

printf '%s\n' "Packaged consumer check passed for configlab $VERSION."
