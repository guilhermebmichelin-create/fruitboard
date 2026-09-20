# Issue #148 security activation proposal — 2026-09-20

This is a review-only proposal. It contains configuration text and an activation
sequence, not an active workflow or a repository-setting change. The owner must
approve and perform the GitHub-side activation before any proposed workflow is
copied into `.github/workflows` or added to required checks.

## Current read-only snapshot

Snapshot source: public repository `guilhermebmichelin-create/fruitboard`,
`main` at `66fcd2a92d4e5a75f016528ff64f46dcf01f45b5`.

- CodeQL default setup: `not-configured`; the API advertises Actions,
  JavaScript/TypeScript, Python, and Rust language support.
- Dependabot security alerts: disabled (`403` from the alerts endpoint).
- Code-scanning alerts: no analysis is configured (`404` from the alerts
  endpoint).
- Secret scanning, non-provider patterns, push protection, and validity checks:
  disabled in the repository settings.
- Branch protection is unchanged: strict required checks are the existing
  `docs-policy`, `client`, `rust-portable`, `migration`, `windows-foundation`,
  `security`, `windows-packaging-smoke`, `enumeration-windows`,
  `filesystem-watcher-windows`, and `scan-execution-windows`. There are no
  branch rulesets, and no review-count or code-owner requirement is proposed by
  this item.
- Existing executable baseline remains the repository privacy gate, high-level
  `pnpm audit`, lockfile review, and RustSec checks. Dependabot update schedules
  remain unchanged.

The absence of alerts is not evidence of a clean scan while the products are
disabled.

## Proposed configuration

### 1. CodeQL — prefer default setup

The owner should first enable CodeQL default setup for `main`, selecting
JavaScript/TypeScript and Rust and retaining the default query suite and weekly
schedule. Default setup should be observed on one normal pull request before
any check is required by branch protection.

If default setup cannot represent the repository’s desired policy, the following
is the proposed advanced workflow. It is intentionally shown here, outside
`.github/workflows`, until approved:

```yaml
name: CodeQL
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  schedule:
    - cron: "41 4 * * 1"
  workflow_dispatch:
permissions:
  contents: read
concurrency:
  group: codeql-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true
jobs:
  analyze:
    name: analyze (${{ matrix.language }})
    runs-on: ubuntu-latest
    timeout-minutes: 30
    permissions:
      actions: read
      contents: read
      packages: read
      security-events: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - language: javascript-typescript
            build-mode: none
          - language: rust
            build-mode: none
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          persist-credentials: false
      - uses: github/codeql-action/init@b96794f015dfd88f77b49b1c93e0fa7110f94c63
        with:
          languages: ${{ matrix.language }}
          build-mode: ${{ matrix.build-mode }}
      - uses: github/codeql-action/analyze@b96794f015dfd88f77b49b1c93e0fa7110f94c63
        with:
          category: /language:${{ matrix.language }}
```

The action SHAs above are the reviewed immutable pins in the proposal branch;
the owner should re-check their provenance and current support policy at review
time. No release or signing secret is exposed to this job.

### 2. Dependency review — pull requests only

If the owner chooses an explicit workflow instead of a product-default rule,
the proposed file is:

```yaml
name: Dependency Review
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
concurrency:
  group: dependency-review-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true
jobs:
  dependency-review:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          persist-credentials: false
      - uses: actions/dependency-review-action@a1d282b36b6f3519aa1f3fc636f609c47dddb294
        with:
          fail-on-severity: high
          comment-summary-in-pr: never
```

The current lockfile/privacy/RustSec checks remain in place. Dependency review
must first run as an informational/non-required check on a normal pull request;
only then can the owner decide whether it should become a required context.

### 3. Secret scanning and push protection

The intended repository settings diff is:

```json
{
  "security_and_analysis": {
    "secret_scanning": { "status": "enabled" },
    "secret_scanning_push_protection": { "status": "enabled" }
  }
}
```

This JSON is descriptive only; no REST update was made. Secret validity checks
remain optional and should be enabled only after the owner decides whether the
additional provider calls fit the project’s privacy model. The owner should
also review the repository’s custom-pattern choice before enabling it.

## Executable post-activation verification (owner-run; not performed here)

The following procedures are deliberately separate from this proposal. They
must run in an isolated scratch repository or branch outside the Fruitboard
working tree, with no Fruitboard dependency, release credential, or required
check change. Each procedure has a positive case, a clean control, an expected
observable result, and a cleanup step. A missing product, missing query, or
missing advisory graph is recorded as **inconclusive**, never as a pass.

### CodeQL positive and clean pair

1. After CodeQL is enabled, create an isolated scratch branch/repository with
   one JavaScript file and no application dependency. Use the documented
   `js/code-injection` source shape:

   ```js
   function run() {
     const value = document.location.href.substring(7);
     eval(value);
   }
   ```

2. Run the selected CodeQL analysis on that branch. Capture the workflow URL,
   commit SHA, SARIF artifact or alert URL, query/rule id, file, and line. The
   expected finding is `ruleId: js/code-injection` with the supported
   code-injection path and high/error severity. This is a finding-level check,
   not merely proof that a job completed.
3. Make the clean control by replacing the `eval` with a non-executing use of
   the value, for example `document.body.textContent = value`, and run the
   same analysis on a new commit. The expected result is a completed analysis
   with no `js/code-injection` result. If the query is not selected, the
   analysis is not completed, or the alert cannot be tied to the scratch
   commit, record the exact limitation as inconclusive.
4. Export only the minimum SARIF/alert metadata needed for the evidence, then
   delete the scratch branch/repository and artifacts. Do not copy the
   synthetic source into Fruitboard.

The supported query is documented at
[CodeQL's JavaScript code-injection query help](https://codeql.github.com/codeql-query-help/javascript/js-code-injection/).

### Dependency Review vulnerable and clean pair

1. In a separate scratch pull request, add only a package manifest and lock
   file. Use `lodash@4.17.20` as the vulnerable control and do not install
   `node_modules` or run application code. If generating the lock file locally,
   use `npm install --package-lock-only --ignore-scripts --no-audit
   lodash@4.17.20` in that scratch checkout only.
2. Run the proposed dependency-review check with `fail-on-severity: high`.
   The expected result is a blocked/failing PR check that names lodash,
   severity high, and the authoritative advisory
   [GHSA-35jh-r3h4-6jhm](https://github.com/advisories/GHSA-35jh-r3h4-6jhm)
   (CVE-2021-23337; affected versions below 4.17.21).
3. Change only the scratch dependency to `lodash@4.17.21`, regenerate the
   lockfile, and rerun the check. The expected result is a completed clean
   check with no high-severity advisory. If the action cannot resolve the
   graph or does not compare the before/after dependency state, record that
   limitation as inconclusive.
4. Delete the scratch branch, pull request, lockfiles, and retained artifacts
   after recording the check URLs and dependency graph. Fruitboard itself must
   never contain the vulnerable package as a test fixture.

### Push-protection capability check

A safe positive push-protection fixture is **currently unavailable** for this
proposal. GitHub documents generic patterns such as `rsa_private_key` for
secret scanning, but supported user-alert patterns and push-protection
patterns are not identical; a fake `ghp_`-style value is not an authoritative
detector test and a real credential must not be created or committed.

The resolution path is: before activation, consult the current supported
pattern/capability table or GitHub Support for a documented non-secret test
fixture, or obtain an owner-approved provider test credential in a throwaway
repository. If and only if an official fixture is available, push one small
scratch commit and record the expected remote rejection before the commit is
accepted, including detector/pattern, file, commit, and server message. An
owner-approved bypass may be tested separately only to verify the audit event;
remove the branch and scratch data afterward and revoke a provider test
credential if one was used. If no official fixture exists, leave this check
explicitly unavailable and do not claim push protection was verified.

The capability boundary is described in GitHub's
[secret-scanning scope](https://docs.github.com/en/code-security/reference/secret-security/secret-scanning-scope)
and [supported secret-scanning patterns](https://docs.github.com/en/code-security/reference/secret-security/supported-secret-scanning-patterns).

## Rollout and validation

1. Owner records approval on #148 and enables CodeQL default setup, or approves
   the advanced workflow above as a separate focused PR.
2. Owner enables secret scanning and push protection in repository settings.
3. Land dependency review as an informational/non-required check, then run the
   vulnerable/clean scratch pair above and retain both check results.
4. Run the CodeQL positive/clean pair above and retain the exact rule id and
   SARIF/alert evidence. Confirm the existing required checks remain present
   and green.
5. Run the push-protection capability check only if an official safe fixture is
   available; otherwise retain the documented unavailable result and its
   resolution path.
6. If the owner wants either new check required, update branch protection only
   after the check has a stable successful run and record the exact context.

Local non-activation validation for a proposal PR:

```powershell
pnpm privacy:check
pnpm lint:docs
git diff --check
```

The action snippets should additionally be checked with the pinned `actionlint`
binary before copying them into a workflow. Record the binary version, action
SHA provenance, and clean output. No workflow file is present in this proposal
branch, so this branch cannot claim an Actions run for the snippets.

## Owner decisions still required

- Enable CodeQL default setup or approve the advanced workflow.
- Enable secret scanning and push protection, and decide on validity checks and
  custom patterns.
- Decide whether dependency review becomes a required check after observation.
- Decide whether any new security context belongs in branch protection. No
  current required context should be removed or renamed as part of this item.

References: [CodeQL setup](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/configure-code-scanning/configure-code-scanning), [dependency review](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/manage-your-dependency-security/configure-dependency-review-action), [secret scanning](https://docs.github.com/en/code-security/how-tos/secure-your-secrets/detect-secret-leaks/enable-secret-scanning), [push protection](https://docs.github.com/en/code-security/how-tos/secure-your-secrets/prevent-future-leaks/enable-push-protection), and [protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches).
