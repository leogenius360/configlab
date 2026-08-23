# Security policy

Please do not publish credentials, secret configuration values, private keys,
or exploitable vulnerability details in public issues.

For a suspected vulnerability, use the repository's GitHub **Security** tab and
select **Report a vulnerability**. Private vulnerability reporting must be
enabled before the repository is made public. Do not include exploit details in
a public issue. Reports are reviewed by Dominic Maabobra Tuolong, the sole
current maintainer. Include:

- affected ConfigLab version;
- impact and threat model;
- a minimal reproduction if safe to share;
- whether secret material may have been exposed;
- any suggested mitigation.

The project's secret wrappers and redacted diagnostic APIs reduce accidental
disclosure but do not provide secure-memory erasure or secret-store isolation.
See [`docs/SECURITY.md`](docs/SECURITY.md) for the technical security model.
