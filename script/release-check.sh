#!/bin/sh
set -eu

ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

fail() {
  echo "release check error: $*" >&2
  exit 1
}

VERSION=$(awk -F '"' '/^version = "/ { print $2; exit }' Cargo.toml)
[ -n "$VERSION" ] || fail "could not determine root package version"

grep -q '^name = "configlab"$' Cargo.toml || fail "root package must be named configlab"
grep -q '^name = "configlab-macros"$' macros/Cargo.toml || fail "macro package must be named configlab-macros"
grep -Fq "configlab-macros = { version = \"=$VERSION\"" Cargo.toml || fail "macro dependency version is not synchronized"
grep -Fq "## [$VERSION]" CHANGELOG.md || fail "CHANGELOG is missing the $VERSION release section"

for file in LICENSE CHANGELOG.md CONTRIBUTING.md CODE_OF_CONDUCT.md SECURITY.md docs/COMPATIBILITY.md docs/RELEASING.md docs/ECOSYSTEM.md; do
  [ -f "$file" ] || fail "required release file is missing: $file"
done

if grep -R -n -E 'config-system|config_system|alpha\.(9|10|11|12|13|14)|MIGRATION_ALPHA' \
  . --exclude-dir=.git --exclude-dir=target --exclude=Cargo.lock --exclude=release-check.sh; then
  fail "pre-public project identity/history remains"
fi

printf '%s\n' "Release metadata check passed for ConfigLab $VERSION."
