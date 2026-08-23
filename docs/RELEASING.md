# Release process

ConfigLab releases are produced from a clean Git working tree after the complete
quality gate succeeds.

## 1. Prepare

1. Choose the next version according to the compatibility policy.
2. Update the workspace version and the exact `configlab-macros` dependency.
3. Move relevant entries from `CHANGELOG.md`'s `Unreleased` section into the new
   version section.
4. Confirm package metadata, documentation, and examples describe the released
   API rather than planned functionality.
5. Ensure `repository` metadata in both publishable manifests points to the
   canonical GitHub repository.

Run:

```bash
./script/refresh-lock.sh
```

Review `Cargo.lock` rather than accepting dependency changes mechanically.

## 2. Verify

```bash
cargo fmt --all
./script/check.sh
./script/release-check.sh
```

The gate must be clean on Rust 1.97.1. Do not publish from a tree with skipped
checks, compiler warnings, rustdoc warnings, or failing package-consumer tests.

## 3. Review package contents

`script/package-check.sh` assembles both `.crate` archives and validates an
independent consumer. Before publishing, also inspect:

```bash
cargo package --list -p configlab-macros
cargo package --list -p configlab
```

No workspace-only examples, scripts, build output, or unpublished internal
history should leak into the main package. The automated gate evaluates actual
archive file entries and reports the exact offending paths; archive directory
entries alone are not treated as leaked package content.

## 4. Commit and tag

Commit the release preparation, then create an annotated tag:

```bash
git tag -a "v$VERSION" -m "ConfigLab $VERSION"
```

Set `VERSION` to the version being released before running the command.

## 5. Publish

The proc-macro companion must be published before the main crate because the
main crate declares an exact dependency on it.

```bash
cargo publish -p configlab-macros
cargo publish -p configlab
```

Wait for the companion crate to become available in the registry before
publishing the main crate if crates.io has not indexed it yet.

## 6. Post-release

- Push the release commit and annotated tag.
- Verify the crates.io and docs.rs pages.
- Confirm the README examples render correctly.
- Restore an empty `Unreleased` section if the release commit consumed it.
- Record any release-process issue as a follow-up rather than silently changing
  the published artifact.
