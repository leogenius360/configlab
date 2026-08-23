# ConfigLab pitch video script

## 01. ConfigLab

Configuration sits on every critical path, yet most teams still treat it as glue code. ConfigLab makes configuration deterministic, typed, explainable, and safe to evolve—from ordinary startup files to deliberate runtime change.

## 02. Configuration is fragmented

Applications combine defaults, files, environment variables, command-line flags, deployment systems, and eventually remote providers. Without one model, teams cannot confidently answer which value won, why it won, whether it is valid, or whether it can change safely while the process is running.

## 03. Separate source, selector, and layer

ConfigLab starts with a clean separation. Source answers where input came from. Selector answers when it applies. Layer answers how much authority it has. Once those concerns stop leaking into one another, resolution becomes predictable and every effective value can carry an explanation.

## 04. One semantic pipeline

The product is one semantic pipeline. Applications define a typed contract. Physical sources normalize into source-neutral logical inputs. One resolver applies layers, selectors, merge and removal rules. Complete-value validation runs before the application receives either a typed value or a refresh report.

## 05. Determinism is a feature

Determinism is the foundation. Source type never secretly decides precedence. Selector specificity only compares peers inside one layer. Equal-authority conflicts fail loudly. Collection behavior belongs to the schema. Provenance explains defaults, winners, overrides, and shadowed inputs.

## 06. Safe runtime refresh

Runtime configuration uses the same plan and resolver as startup. A refresh rereads every registered physical source, combines any retained source-neutral external inputs, and validates the complete result. A successful refresh immediately replaces the current typed value; a parse, resolution, or validation failure leaves it unchanged.

## 07. Explicit trust boundaries

Production trust comes from boundaries. ConfigLab owns resolution, validation, redaction, transactional in-process refresh, current provenance, and redaction-aware change reports. The host decides when to refresh and owns remote transport, secret retrieval, distributed rollout, history, and arbitrary external side effects.

## 08. Simple developer experience

The common path remains small: derive a typed configuration, add an optional file, environment, and arguments, then load. The same schema handles TOML, JSON, and YAML. When needs grow, the builder can become a reusable runtime plan without introducing a second semantic system.

## 09. Proof, not promises

This is not a slide-only architecture. The repository carries one hundred forty library and example behavioral tests, seven proc-macro unit tests, five public-API applications, and exactly one semantic resolver. The gate denies warnings, checks isolated features, builds release artifacts, and compiles a fresh packaged consumer.

## 10. Who wins

The leverage compounds. Application developers remove bespoke parsing and validation. Library authors publish source-neutral requirements. Operators get effective state and provenance. Platform teams standardize configuration semantics before committing to any one storage or control-plane technology.

## 11. Small core, growing ecosystem

The roadmap protects the core. Today delivers typed definitions, deterministic resolution, explainability, and explicit runtime refresh. Next comes introspection, reusable components, testing tools, and provider patterns. Remote transports and control planes remain separate projects when real adoption validates the demand.

## 12. The ask

The invitation is simple: adopt the alpha in real Rust services, challenge the model with the edge cases that break ordinary loaders, and help shape reusable components and integrations. ConfigLab can turn configuration from repeated glue code into dependable infrastructure.
