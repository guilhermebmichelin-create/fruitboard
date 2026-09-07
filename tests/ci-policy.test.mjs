import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readRootFile = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

const workflow = readRootFile(".github/workflows/foundation.yml");

test("foundation CI exposes stable, always-present checks", () => {
  for (const job of [
    "docs-policy",
    "client",
    "rust-portable",
    "migration",
    "windows-foundation",
    "security",
  ]) {
    assert.match(workflow, new RegExp(`^  ${job}:\\n    name: ${job}$`, "m"));
  }

  assert.match(workflow, /^  pull_request:$/m);
  assert.match(workflow, /^  push:$/m);
  assert.match(workflow, /^  workflow_dispatch:$/m);
  assert.doesNotMatch(workflow, /pull_request_target|\n\s+paths(?:-ignore)?:/);
  assert.doesNotMatch(workflow, /^    if:/m);
  assert.doesNotMatch(workflow, /continue-on-error:/);
});

test("workflow permissions and third-party execution fail closed", () => {
  assert.match(workflow, /^permissions:\n  contents: read$/m);
  assert.match(workflow, /cancel-in-progress: true/);
  assert.doesNotMatch(workflow, /\bsecrets[.:]/);
  assert.doesNotMatch(workflow, /upload-artifact|release|publish/i);

  const actionReferences = [
    ...workflow.matchAll(/^\s*uses:\s*([^\s#]+)$/gm),
  ].map((match) => match[1]);
  const approvedActionReferences = new Set([
    "actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
    "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
    "actions/setup-node@820762786026740c76f36085b0efc47a31fe5020",
    "astral-sh/setup-uv@20cfd1bf945f4377ade1205e4dbc17946fc9a30d",
  ]);
  assert.ok(actionReferences.length > 0);
  for (const reference of actionReferences) {
    assert.match(
      reference,
      /^[^@]+@[0-9a-f]{40}$/,
      `${reference} must use an immutable commit SHA`,
    );
    assert.ok(
      approvedActionReferences.has(reference),
      `${reference} must be explicitly reviewed and allowlisted`,
    );
  }
  assert.deepEqual(new Set(actionReferences), approvedActionReferences);

  const checkoutCount = actionReferences.filter((reference) =>
    reference.startsWith("actions/checkout@"),
  ).length;
  const disabledCredentialCount = (
    workflow.match(/^\s+persist-credentials: false$/gm) ?? []
  ).length;
  assert.equal(checkoutCount, 7);
  assert.equal(disabledCredentialCount, checkoutCount);
});

test("CI commands cover locked client, portable storage, migrations, and Windows", () => {
  assert.match(workflow, /pnpm install --frozen-lockfile --ignore-scripts/);
  assert.match(
    workflow,
    /pnpm\.cmd install --frozen-lockfile --ignore-scripts/,
  );
  assert.match(workflow, /pnpm --filter @fruitboard\/client build/);
  assert.doesNotMatch(workflow, /pnpm --filter @fruitboard\/desktop build/);
  assert.match(
    workflow,
    /cargo clippy -p fruitboard-storage --all-targets --locked -- -D warnings/,
  );
  assert.match(workflow, /cargo test -p fruitboard-storage --locked/);
  assert.match(
    workflow,
    /upgrades_every_supported_fixture_and_preserves_existing_rows/,
  );
  assert.match(
    workflow,
    /killed_migration_recovers_the_original_committed_database/,
  );
  assert.match(workflow, /pnpm\.cmd check/);
  assert.match(workflow, /uv python install 3\.11\.16/);
  assert.match(workflow, /uv sync --frozen --python 3\.11\.16/);
});

test("security checks cover repository privacy and both dependency locks", () => {
  const packageJson = JSON.parse(readRootFile("package.json"));
  const auditConfig = readRootFile(".cargo/audit.toml");

  assert.match(packageJson.scripts.check, /pnpm privacy:check/);
  assert.match(workflow, /node scripts\/verify-repository-privacy\.mjs/);
  assert.match(workflow, /pnpm audit --audit-level high/);
  assert.match(
    workflow,
    /cargo install cargo-audit --locked --version 0\.22\.2/,
  );
  assert.match(workflow, /cargo audit --file Cargo\.lock/);
  assert.match(auditConfig, /deny = \["warnings"\]/);
  assert.deepEqual((auditConfig.match(/RUSTSEC-\d{4}-\d{4}/g) ?? []).sort(), [
    "RUSTSEC-2024-0370",
    "RUSTSEC-2024-0411",
    "RUSTSEC-2024-0412",
    "RUSTSEC-2024-0413",
    "RUSTSEC-2024-0414",
    "RUSTSEC-2024-0415",
    "RUSTSEC-2024-0416",
    "RUSTSEC-2024-0417",
    "RUSTSEC-2024-0418",
    "RUSTSEC-2024-0419",
    "RUSTSEC-2024-0420",
    "RUSTSEC-2024-0429",
    "RUSTSEC-2025-0075",
    "RUSTSEC-2025-0080",
    "RUSTSEC-2025-0081",
    "RUSTSEC-2025-0098",
    "RUSTSEC-2025-0100",
  ]);
});

test("repository ownership and update automation use real maintainers and ecosystems", () => {
  const codeowners = readRootFile(".github/CODEOWNERS");
  const dependabot = readRootFile(".github/dependabot.yml");

  assert.match(codeowners, /^\* @guilhermebmichelin-create$/m);
  assert.match(codeowners, /^\/\.github\/ @guilhermebmichelin-create$/m);
  assert.match(
    codeowners,
    /^\/crates\/storage-sqlite\/migrations\/ @guilhermebmichelin-create$/m,
  );
  assert.doesNotMatch(codeowners, /@(?:owner|team|placeholder)\b/i);

  for (const ecosystem of ["npm", "cargo", "github-actions"]) {
    assert.match(dependabot, new RegExp(`package-ecosystem: ${ecosystem}`));
  }
  assert.equal((dependabot.match(/package-ecosystem:/g) ?? []).length, 3);
});

test("issue forms require scoped, privacy-aware reports", () => {
  const config = readRootFile(".github/ISSUE_TEMPLATE/config.yml");
  const bug = readRootFile(".github/ISSUE_TEMPLATE/bug.yml");
  const feature = readRootFile(".github/ISSUE_TEMPLATE/feature.yml");

  assert.match(config, /blank_issues_enabled: false/);
  assert.match(config, /security\/advisories\/new/);
  for (const form of [bug, feature]) {
    assert.match(form, /Privacy confirmation/);
    assert.match(form, /personal paths/);
    assert.match(form, /required: true/);
  }
  assert.match(bug, /Minimal reproduction/);
  assert.match(feature, /Non-goals and deferrals/);
  assert.match(feature, /Accessibility and application states/);
});
