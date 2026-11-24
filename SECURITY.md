# Plugin Store — Security Posture (B2 Checklist Compliance)

This document describes the security posture of the Plugin Store CI/CD
pipeline as enforced by the workflow YAML in `.github/workflows/` and
records the GitHub Settings-level configuration required for full
checklist compliance.

The repository code in this commit covers everything that can be
expressed in YAML / Rust source. Items marked **[Settings]** must be
applied via the GitHub repository or organization settings UI.

---

## B2.1 — Triggers and Permissions

| Item | Status | Where |
|---|---|---|
| All workflows top-level `permissions: read-all` (or read-only) | ✅ enforced | every `.github/workflows/*.yml` |
| Write permissions declared per-job, not top-level | ✅ enforced | `plugin-publish.yml`, `release.yml`, `update-registry.yml` |
| `pull_request_target` workflows pass A1 grep checks (no `${{ github.event.* }}` in `run:` blocks) | ✅ enforced | A1 backport commit |
| Secret-bearing jobs declare `environment:` | ✅ enforced | `ai-review`, `plugin-review`, `production` |
| **[Settings]** Repo Settings → Actions → Fork PR workflows: *Require approval for all outside collaborators* | ⚠️ to be set | GitHub Settings UI |

## B2.2 — Secrets Isolation

| Item | Status | Where |
|---|---|---|
| LLM keys in dedicated environment, separated from build secrets | ✅ enforced | `environment: ai-review` (only `ANTHROPIC_API_KEY` / `OPENROUTER_API_KEY`) — different from `production` |
| **[Settings]** Provider-side IP allowlist + rate limit | ⚠️ to be set | Anthropic console / OpenRouter dashboard |
| **[Settings]** Test/staging/prod credentials not reused | ⚠️ to be set | GitHub environments — define separate secrets per environment |
| No secret written to `set-output` / `GITHUB_ENV` (anti-chain-exploitation) | ✅ verified | `grep set-output` and `grep GITHUB_ENV` show 0 hits with secret values |

## B2.3 — Runner Type

| Item | Status | Where |
|---|---|---|
| All `runs-on:` are GitHub-hosted (each job runs on a fresh ephemeral VM) | ✅ enforced | values: `ubuntu-latest`, `ubuntu-22.04`, `macos-latest`, `windows-2022`, matrix `${{ matrix.os }}` |
| Self-hosted runners | ❌ not used | `grep -E 'runs-on:.*self-hosted'` returns 0 |
| **[Settings]** If a self-hosted runner is ever introduced: take it offline during the PoC window, re-image (do not reuse), enroll into CIS Benchmark / Falcon | ⚠️ policy | only triggered if any self-hosted runner is ever added |

## B2.4 — Third-Party Action Governance

| Item | Status | Where |
|---|---|---|
| All third-party actions pinned to a 40-char SHA, not a floating tag | ✅ enforced | 59 SHA pins applied across all 9 workflows; `grep '@[0-9a-f]\{40\}'` matches every `uses:` |
| Pin comment format `# <original-tag>` for human readability | ✅ enforced | every pin retains its original tag in a trailing comment |
| **[Settings]** Org → Actions → *Allow specified actions and reusable workflows only* with the inventory below | ⚠️ to be set | GitHub Settings UI |
| **[Settings]** Dependabot alerts enabled for `.github/workflows/` | ⚠️ to be set | Settings → Code security |
| **[Settings]** GitGuardian / secret-scan enabled at repo or org level | ⚠️ to be set | Settings → Code security |

### Approved third-party action inventory (whitelist for org settings)

```
actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830                    # v4
actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5                 # v4
actions/create-github-app-token@d72941d797fd3113feb6b93fd0dec494b13a2547  # v1
actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093        # v4
actions/github-script@f28e40c7f34bde8b3046d885e986cb6290c5673b            # v7
actions/setup-go@40f1582b2485089dde7abd97c1529aa768e1baff                 # v5
actions/setup-python@a26af69be951a213d495a4c3e4e4022e16d87065             # v5
actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02          # v4
dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8           # stable
mlugg/setup-zig@53fc45b17fe98b52f92ee5ea08ff48a85a3e7eb7                  # v1
oven-sh/setup-bun@0c5077e51419868618aeaa5fe8019c62421857d6                # v2
softprops/action-gh-release@3bb12739c298aeb8a4eeaf626c5b8d85266b0e65      # v2
```

Any pull request that introduces a `uses:` line outside this whitelist
must go through a separate security review.

---

## How to update SHA pins

When a third-party action releases a new version, update the SHA *and*
the comment in lock-step. Recommended commands:

```bash
# Find current pinned version + tag
grep -hoE 'uses: [^ ]+' .github/workflows/*.yml | sort -u

# Resolve a new SHA for a desired tag
gh api repos/<owner>/<repo>/git/refs/tags/<tag> --jq '.object.sha'

# Update the workflow file
sed -i 's|<repo>@<old-sha>|<repo>@<new-sha>|' .github/workflows/*.yml
```

---

## Related documents

- `docs/FOR-DEVELOPERS.md` — external developer plugin-submission guide
- `Plugin-Store-Security-Architecture-Evolution-EN.md` — architecture
  evolution + threat model (input for the Security Team's deliverable)
