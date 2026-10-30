# Unicode dependency updates and stored locator keys

PR #360 retains `unicode-normalization = "=0.1.25"` and its Unicode 17.0.0
normalization data. `LocatorKeyV1` pairs this NFC operation with the vendored
Unicode 17 simple-fold table in `crates/filesystem-enumeration/src/simple_fold.rs`.
These operations contribute to stored filename keys, not just display text.

The proposed 0.1.26 dependency uses Unicode 18.0.0, as declared in its
[generated table](https://github.com/unicode-rs/unicode-normalization/blob/v0.1.26/src/tables.rs).
Updating it alone would break the documented pairing. The upgrade is deferred;
this PR adds a runtime-data-version regression assertion and makes Dependabot
honor the existing compatibility boundary. No stored keys or project files change.

A future upgrade must review the normalization and simple-fold data together,
document whether newly assigned characters change existing stored keys, and
define compatibility, reconciliation and rollback behavior before updating the
pin. It must then update the generator/provenance, contract and regression cases
for any changed equivalence classes. Removing the Dependabot exclusion belongs
to that coordinated change.

Validation requires the Windows enumeration tests and warning-denied Clippy,
script/policy tests, scoped formatting, Markdown lint and normal updated-head CI.
No installed-app or performance qualification is claimed by this guard change.
