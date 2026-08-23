#!/bin/sh
set -eu

ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

fail() {
  echo "architecture error: $*" >&2
  exit 1
}

[ ! -d crates ] || fail "/crates runtime layout is not permitted"
[ ! -d dogfood ] || fail "/dogfood directory is not permitted"
[ -d macros ] || fail "/macros proc-macro companion is missing"
[ -d src/source ] || fail "physical sources must remain split under /src/source"
[ -d src/resolution ] || fail "resolver responsibilities must remain split under /src/resolution"
[ -d src/definition ] || fail "definition responsibilities must remain split under /src/definition"
[ ! -f src/source.rs ] || fail "legacy monolithic src/source.rs must not return"
[ ! -f macros/src/derive.rs ] || fail "legacy monolithic macros/src/derive.rs must not return"
[ -d examples/basic-app ] || fail "basic-app example is missing"
[ -d examples/cli-tool ] || fail "cli-tool example is missing"
[ -d examples/contextual-service ] || fail "contextual-service example is missing"
[ -d examples/runtime-service ] || fail "runtime-service example is missing"
[ -d examples/platform-integration ] || fail "platform-integration example is missing"
[ -d tests/kernel ] || fail "kernel acceptance tests must remain split under /tests/kernel"
[ -d tests/resolution ] || fail "resolver acceptance tests must remain split under /tests/resolution"
[ -d tests/facade ] || fail "facade acceptance tests must remain split under /tests/facade"
[ -f docs/RESOLUTION.md ] || fail "resolution semantics guide is missing"
[ -f docs/TESTING.md ] || fail "testing guide is missing"
[ -f docs/COMPATIBILITY.md ] || fail "compatibility policy is missing"
[ -f docs/RELEASING.md ] || fail "release process guide is missing"
[ -f docs/ECOSYSTEM.md ] || fail "ecosystem direction guide is missing"
[ -f docs/RUNTIME_LIFECYCLE.md ] || fail "runtime lifecycle guide is missing"
[ -f CHANGELOG.md ] || fail "CHANGELOG is missing"
[ -f CONTRIBUTING.md ] || fail "CONTRIBUTING guide is missing"
[ -f CODE_OF_CONDUCT.md ] || fail "code of conduct is missing"
[ -f .gitlab-ci.yml ] || fail "GitLab CI configuration is missing"
[ -f .github/workflows/ci.yml ] || fail "GitHub Actions configuration is missing"
[ -f LICENSE ] || fail "BSD 3-Clause license file is missing"
[ -x script/release-check.sh ] || fail "release-check script is missing or not executable"
[ -x script/package-check.sh ] || fail "package-check script is missing or not executable"
[ -x script/performance-baseline.sh ] || fail "performance-baseline script is missing or not executable"
[ ! -f tests/kernel_completion.rs ] || fail "legacy mixed kernel_completion.rs test bucket must not return"
[ ! -f tests/integration.rs ] || fail "legacy mixed integration.rs test bucket must not return"
[ ! -f tests/resolution/merge_removal.rs ] || fail "legacy mixed merge_removal.rs test bucket must not return"

# Repository maintenance scripts belong in /script. Crate-native Rust files
# such as build.rs are source code and are not covered by this layout rule.
misplaced_scripts=$(find . -type f \
  \( -name '*.sh' -o -name '*.py' -o -name '*.ps1' -o -name '*.cmd' -o -name '*.bat' \) \
  ! -path './script/*' -print)
[ -z "$misplaced_scripts" ] || fail "maintenance scripts must live under /script: $misplaced_scripts"

# The proc-macro crate is an implementation detail. Examples depend only on the
# main package and must not bypass public APIs.
if grep -R -n -E 'configlab-macros|configlab_macros|__private' examples --include='*.rs' --include='Cargo.toml'; then
  fail "examples must depend only on the public configlab API"
fi

if grep -R -n -E 'config-core|config_core|crates/configlab' src macros examples tests --include='*.rs' --include='Cargo.toml'; then
  fail "legacy package-boundary references remain"
fi

# Pre-1.0 consolidated API intentionally removes the combinatorial source facade.
if grep -R -n -E '\.(optional_file|optional_file_when|optional_file_in|file_when|file_in|ordered_file_in|environment_from|environment_in|arguments_from|env_prefix|env_separator|layers)\(' src examples tests --include='*.rs'; then
  fail "legacy combinatorial builder API remains"
fi

if grep -n -E 'pub fn (optional_file|optional_file_when|optional_file_in|file_when|file_in|ordered_file_in|environment_from|environment_in|arguments_from|env_prefix|env_separator|layers)\b' src/builder.rs; then
  fail "legacy combinatorial builder methods remain defined"
fi

# One semantic resolver only.
resolver_count=$(grep -R -l 'pub struct Resolver' src --include='*.rs' | awk 'END { print NR }')
[ "$resolver_count" = "1" ] || fail "expected exactly one Resolver implementation, found $resolver_count"

# Root lib.rs is navigation/re-export surface, not implementation storage.
lib_lines=$(awk 'END { print NR }' src/lib.rs)
[ "$lib_lines" -le 140 ] || fail "src/lib.rs should remain a small module/re-export surface"

macro_lib_lines=$(awk 'END { print NR }' macros/src/lib.rs)
[ "$macro_lib_lines" -le 80 ] || fail "macros/src/lib.rs should remain an entrypoint, not derive implementation storage"

if grep -R -n -E 'todo!\(|unimplemented!\(|TODO|FIXME' src macros/src examples tests --include='*.rs'; then
  fail "unfinished implementation marker found"
fi

if grep -R -n 'unsafe[[:space:]]*{' src macros/src examples tests --include='*.rs'; then
  fail "unsafe block found"
fi

# Evergreen product docs describe the current product, not the alpha in which a
# paragraph was first introduced. Historical version references belong in
# migration/acceptance/verification records.
if grep -n -E 'Alpha\.10|alpha\.10|Alpha\.11|alpha\.11|Alpha\.12|alpha\.12' \
  docs/API.md docs/ARCHITECTURE.md docs/DEPENDENCIES.md docs/PERFORMANCE.md \
  docs/RESOLUTION.md docs/SECURITY.md docs/TESTING.md; then
  fail "evergreen documentation contains stale alpha-version labels"
fi

if grep -n -E '^pub use path::\{.*(get_path|set_path)' src/resolution/mod.rs; then
  fail "low-level path-tree mutation helpers must remain internal"
fi

if grep -n -E '^[[:space:]]*fn load\(\) -> ConfigBuilder' src/typed.rs; then
  fail "Config::load builder alias must not return"
fi
