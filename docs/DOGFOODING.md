# Dogfooding

ConfigLab's examples are intentionally real workspace consumers of the public
crate rather than privileged in-crate demos.

## basic-app

Exercises the shortest normal path: typed derive, defaults, optional file,
environment, arguments, and a secret reference.

## cli-tool

Exercises a realistic CLI configuration module: optional project file,
environment prefixing, deterministic argument fixtures, collections, validation,
and optional credentials.

## contextual-service

Exercises nested service configuration, contextual and regional selectors,
cross-section validation, conditional requirements, redacted output, and
explainability/provenance.

## runtime-service

Exercises the complete in-process refresh lifecycle with a temporary optional
file: repeated acquisition, parse and validation failure recovery, immediate
validated replacement, redacted changes, provenance, and source removal. It
owns no background thread or async runtime.

## platform-integration

Exercises the advanced source-neutral API with default features disabled:
hand-written schemas, reusable component composition, logical-input adapters,
selectors, merge/removal policies, diagnostics, provenance, redaction, and
separate operational verification.

## Packaged consumer

`script/package-check.sh` creates both `.crate` archives, extracts them into a
temporary directory, creates a new independent application, and compiles/runs it
against the package contents. This catches workspace-only assumptions that the
examples cannot.

## How dogfooding should influence the API

Dogfooding may justify facade improvements when real consumers expose repeated
friction. It should not be used as an excuse to weaken resolver invariants or to
introduce speculative remote/plugin/control-plane capabilities.
