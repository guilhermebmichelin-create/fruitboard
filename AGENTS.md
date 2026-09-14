# Fruitboard agent instructions

Read [DEVELOPMENT.md](DEVELOPMENT.md) before planning or changing this project.
Apply its [build and disk-space rules](DEVELOPMENT.md#local-build-and-disk-space-rules)
when generating agent prompts, assigning work, running checks, and handing off
completed work. These instructions do not themselves request additional agents.

## Agent prompts and coordination

Every generated implementation or validation prompt must include:

- The owned files/worktree and the exact source boundary to validate.
- Whether a local build is needed, which checks are required, and which existing
  CI results apply to unchanged code.
- The build-cache path and its single owner. Default to one primary development
  cache and one reusable temporary validation cache per machine, not one cache
  per agent or PR.
- A disk-space preflight: maintain at least **30 GiB free** on each volume used
  for builds, temporary files, fixtures, and evidence. Account for expected
  additional output, not just the space available before starting.
- The exclusive window for heavyweight builds or installed tests. Qualification
  measurements run alone, with other agent/build/test workloads quiescent.
- Evidence storage outside disposable compiler caches and the cleanup/handoff
  responsibilities for obsolete generated files.

Documentation-only and read-only tasks may state that no local build/cache is
needed. Do not add builds simply to satisfy the prompt template.

## Build and cleanup boundaries

- Inspect existing worktrees and caches before creating new ones. Keep separate
  source worktrees where useful, but reuse a small, explicitly owned cache set.
- Run at most one heavyweight local build at a time. Never share a writable
  Cargo target directory between concurrent tasks. Set `CARGO_TARGET_DIR`
  explicitly when using the temporary validation cache.
- Never hardlink-clone a writable Cargo output into another worktree, cache, or
  retained-evidence location (`fs.link`, `mklink /H`, or an equivalent). Cargo
  may rewrite that output in place, so the shared file identity can mutate a
  historical binary. Reuse the one selected writable cache sequentially under
  its named owner; retain a binary only as an independently copied file and
  record its source/build provenance and hash.
- If the reserve cannot be maintained, reuse existing compatible artifacts,
  use another suitable local volume, or reclaim verified obsolete compiler
  intermediates within the authorized scope. Continue independent lightweight
  work; report a specific blocker only if the build still cannot proceed.
- Keep required checks intact. Existing CI for an unchanged source tree can
  avoid redundant local builds, but does not replace required checks on an
  updated PR head or prove performance qualification.
- Before cleanup, resolve and verify every target's absolute path, reject
  unexpected reparse points, confirm no task is using it, and inventory what
  will be removed. Use native PowerShell `-LiteralPath` file operations on
  Windows; never broad-delete temporary directories or whole worktrees.
- Preserve dirty/untracked source, Git state, dependencies/toolchains, active
  development caches, fixtures, databases, reports, logs, and retained evidence
  binaries. A folder named `target` is not sufficient proof that everything
  inside it is disposable. Do not use `git clean -fdx` or a blanket `cargo clean`
  as an unreviewed cleanup shortcut.
- After an authorized cleanup, verify worktree state and protected files,
  measure actual reclaimed disk space, and report any rebuild cost. Account
  for hardlinks when estimating reclaimable space.
- On completion, identify cache ownership, reusable outputs, obsolete compiler
  intermediates, and evidence that must remain. Do not leave a new permanent
  multi-gigabyte cache for every completed PR.

The read-only storage safeguard can be run before a heavyweight build or
fixture generation:

```powershell
node scripts/preflight-build-storage.mjs --source <checkout> --build <build-output> --cache <cargo-cache> --evidence <retained-evidence> --estimated-output-gib <GiB>
```

It requires all four explicit directories, reports available capacity on each
relevant volume, and fails if the current or estimated post-build capacity
would be below the 30 GiB reserve. It resolves reparse points only along those
explicit paths and scans only the selected cache. A cache hardlink, an
unresolved or external reparse point under that cache, an unreadable entry, or
a bounded scan that cannot finish is reported as unsafe/uncertain; an unchecked
cache is never certified as isolated. Source/evidence, build/evidence, and
cache/evidence overlaps are unsafe after resolution. A normal build/cache
directory nested below the source checkout is allowed, while a cache containing
the source is not. Unrelated read-only toolchain junctions outside the selected
cache are not blanket-rejected or recursively scanned.

Each writable cache must have one explicitly named owner and an absolute path
recorded in the task handoff. Sequential reuse still requires that owner to
coordinate the exclusive write window. This read-only preflight is a capacity
and path/hardlink check; it does not prove continuing exclusivity and does not
replace host coordination.

These are workflow requirements, not an installed automatic cleanup service.
They do not change benchmark acceptance targets or authorize deletion of user
data. Explicit user instructions take precedence.
