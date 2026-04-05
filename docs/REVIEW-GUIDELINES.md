# Review Guidelines

This document describes the review process that every plugin submission to the OKX Plugin Store must pass before it can be merged. Understanding these standards will help you submit clean plugins and avoid delays.

---

## 1. Review Process Overview

Every pull request goes through four sequential phases:

```
Submit PR
    |
    v
+-------------------+
| Phase 1: Lint     |  Automated, ~30 seconds
+-------------------+
    |
    v
+-------------------+
| Phase 2: Build    |  Automated, 1-5 minutes (if applicable)
+-------------------+
    |
    v
+-------------------+
| Phase 3: AI Review|  Automated, 2-5 minutes
+-------------------+
    |
    v
+-------------------+
| Phase 4: Human    |  Manual, 1-3 business days
+-------------------+
    |
    v
 Approved  or  Changes Requested
```

A failure in any phase blocks progression to the next. You will receive inline PR comments explaining what needs to be fixed.

---

## 2. Phase 1: Static Lint

An automated linter validates structural correctness, safety defaults, and metadata consistency. Rules are grouped by category below.

### Lint Rules

| Category | Check | Severity |
|----------|-------|----------|
| **Structure** | `plugin.yaml` exists at plugin root | Error |
| **Structure** | `SKILL.md` exists (for skill-type plugins) | Error |
| **Structure** | `plugin.yaml` contains valid YAML frontmatter | Error |
| **Version Consistency** | Version in `plugin.yaml` matches version declared in `SKILL.md` | Error |
| **Safety Defaults** | `PAUSED=True` is set (plugins must not auto-start) | Error |
| **Safety Defaults** | `PAPER_TRADE=True` is set (live trading must be opt-in) | Error |
| **Safety Defaults** | `DRY_RUN=True` is set (destructive actions must be opt-in) | Error |
| **Python Validation** | All `.py` files pass syntax check (`py_compile`) | Error |
| **URL Checks** | All URLs referenced in plugin files are reachable (HTTP 2xx) | Warning |
| **Category** | Category is one of: `trading`, `defi`, `game`, `prediction`, `data_tools`, `dev_tools`, `others` | Error |
| **License** | License field contains a valid SPDX identifier (e.g., `MIT`, `Apache-2.0`) | Error |

### Severity Levels

- **Error** -- Blocks merge. The PR cannot proceed until the issue is resolved.
- **Warning** -- Advisory. Flagged for awareness but does not block merge.

---

## 3. Phase 2: Build Verification

If your `plugin.yaml` includes a `build` section, the CI system will attempt to compile your plugin from source.

### Supported Languages

| Language | Build Tool | Trigger |
|----------|-----------|---------|
| Rust | `cargo build --release` | `build.lang: rust` |
| Go | `go build` | `build.lang: go` |
| TypeScript | `npm run build` or `tsc` | `build.lang: typescript` |
| Node.js | `npm install && npm run build` | `build.lang: nodejs` |
| Python | Syntax check + dependency install | `build.lang: python` |

### Build Matrix

Builds are tested across platforms defined in `build.targets` (e.g., `x86_64-linux`, `aarch64-darwin`). All target platforms must build successfully for the check to pass.

### When Build Is Skipped

If your plugin has no `build` section in `plugin.yaml` (e.g., it is a pure SKILL.md plugin with no compiled binary), Phase 2 is skipped entirely.

---

## 4. Phase 3: AI Code Review

An AI reviewer performs a structured audit across nine dimensions, producing a detailed report that is posted as a PR comment.

### Nine Audit Dimensions

| # | Dimension | What Is Evaluated |
|---|-----------|-------------------|
| 1 | **Plugin Overview** | Name, version, category, author, license, risk level, and a plain-language summary of what the plugin does. |
| 2 | **Architecture Analysis** | Component structure (skill/binary), SKILL.md organization, data flow, and external dependencies. |
| 3 | **Permission Analysis** | Inferred permissions: onchainos commands used, wallet operations detected, external APIs/URLs called, and chains operated on. |
| 4 | **OnchainOS Compliance** | Whether all on-chain write operations (signing, broadcasting, swaps, approvals, transfers) use the onchainos CLI rather than self-implementing with raw libraries. This is the single most important check. |
| 5 | **Security Assessment** | Application of static security rules, LLM semantic judges, and toxic flow detection (see below). Includes prompt injection scanning, dangerous operation checks, and data exfiltration risk analysis. |
| 6 | **Source Code Review** | Language and build config, dependency audit, code safety checks (hardcoded secrets, undeclared network requests, filesystem access, dynamic code execution, unsafe blocks). Only applies to plugins with source code. |
| 7 | **Code Quality** | Scored on five sub-dimensions: Completeness (25 pts), Clarity (25 pts), Security Awareness (25 pts), Skill Routing (15 pts), and Formatting (10 pts). |
| 8 | **Recommendations** | Prioritized list of actionable improvements. |
| 9 | **Summary and Score** | One-line verdict, merge recommendation, and an overall quality score from 0 to 100. |

### Three-Layer Security Scanning

The security assessment in Dimension 5 uses three complementary detection layers:

#### Layer 1: Static Rules (28 rules)

Pattern-based scanning across four severity levels. The scanner checks for known dangerous patterns without requiring semantic understanding.

| Severity | Count | What Is Detected |
|----------|-------|------------------|
| **Critical** (C01-C09) | 9 | Command injection (pipe to shell), prompt injection keywords, base64/unicode obfuscation, credential exfiltration via environment variables or command substitution, password-protected archive downloads, pseudo-system tag injection, hidden instructions in HTML comments, backtick injection with sensitive paths. |
| **High** (H01-H09) | 9 | Hardcoded secrets (API keys, private keys, mnemonics), instructions to output credentials, persistence mechanisms (cron, launchctl, systemd), access to sensitive file paths (~/.ssh, ~/.aws, ~/.kube), direct financial/on-chain API operations, system file modification, plaintext credential storage in .env files, credential solicitation in chat, signed transaction data in CLI parameters. |
| **Medium** (M01-M08) | 8 | Unpinned package installations, unverifiable runtime dependencies, third-party content fetching without boundary markers, resource exhaustion patterns (fork bombs, infinite loops), dynamic package installation via eval/exec, skill chaining without version pinning, missing untrusted-data boundary declarations, external data field passthrough without isolation. |
| **Low** (L01-L02) | 2 | Agent capability discovery/enumeration attempts, undeclared network communication (raw IPs, DNS lookups, netcat). |

**Judgment logic:**
- Any Critical finding = **FAIL** (blocks merge)
- Any High or Medium finding (without Critical) = **WARN** (flagged for human review)
- Only Low/Info or no findings = **PASS**

#### Layer 2: LLM Semantic Judges (6 judges)

AI-powered semantic analysis that detects threats beyond pattern matching:

| Judge | Severity | What It Detects |
|-------|----------|-----------------|
| **Prompt Injection** | Critical | Hidden instructions that hijack agent behavior, including instruction overrides, pseudo-system tags, encoded payloads, jailbreak attempts, and CLI parameter injection via unsanitized user input. |
| **Malicious Intent** | Critical | Discrepancy between a plugin's stated purpose and its actual behavior -- e.g., a "wallet tracker" that secretly uploads private keys. |
| **Memory Poisoning** | High | Attempts to write to agent memory files (MEMORY.md, SOUL.md) to plant cross-session backdoors that survive restarts. |
| **External Request Notice** | Info/Medium | External API or CLI calls. Rated Info if the plugin declares an untrusted-data boundary; Medium if it does not. |
| **Autonomous Execution Risk** | Info | Operations that could be executed without explicit user confirmation (vague authorization words like "proceed", "handle", "automatically" without confirmation gates). |
| **Financial Scope Assessment** | Info to Critical | Evaluates the financial operation scope: read-only queries (exempt), confirmed writes (Info), unconfirmed writes (High), fully autonomous fund transfers (Critical). |

Results with confidence below 0.7 are automatically discarded.

#### Layer 3: Toxic Flow Detection (5 attack chains)

Combinations of individually lower-severity findings that together form a complete attack chain:

| Flow | Trigger Combination | Severity | Attack Pattern |
|------|---------------------|----------|----------------|
| **TF001** | Sensitive path access + credential exfiltration or undeclared network | Critical | Read credentials from ~/.ssh or ~/.aws, then exfiltrate via HTTP/DNS/netcat. Complete credential theft chain. |
| **TF002** | Prompt injection + persistence mechanism | Critical | Jailbreak the agent's safety guardrails, then register a persistent service (cron/launchctl) that survives reboots. |
| **TF004** | Unverifiable dependency + malicious intent detected | High | Malicious plugin installs additional unverified packages whose postinstall hooks execute attack payloads. |
| **TF005** | Command injection (curl pipe sh) + financial API access | Critical | Remote script (replaceable at any time) combined with financial operations enables unauthorized fund transfers. |
| **TF006** | Missing data boundary (M07/M08) + financial operations (H05) | High | External data (token names, swap routes) enters agent context without isolation; attacker injects instructions via on-chain fields to manipulate transaction parameters. |

### Quality Score Interpretation

| Score | Meaning |
|-------|---------|
| 80-100 | Ready to merge. No significant issues found. |
| 60-79 | Minor issues identified. Likely approved after targeted fixes. |
| Below 60 | Significant concerns. Substantial changes required before re-review. |

---

## 5. Phase 4: Human Review

After automated checks pass, a human reviewer examines the submission.

### Review Focus by Risk Level

| Plugin Risk Level | Review Depth | Reviewer Count |
|-------------------|-------------|----------------|
| Low (read-only, data display) | Standard review of SKILL.md and metadata | 1 reviewer |
| Medium (writes data, calls external APIs) | Detailed review including data flow analysis | 1 reviewer |
| High/Advanced (financial operations, on-chain writes) | Full security audit of all code and instructions | 2 reviewers required |

### What Human Reviewers Focus On

- Accuracy of the AI review report (confirming or overriding AI findings)
- Business logic correctness that AI may miss
- User experience and documentation quality
- Edge cases in financial operations
- Consistency with existing Plugin Store standards

### SLA

Human review is completed within **1 to 3 business days** of passing Phase 3. Complex or high-risk plugins may take longer if additional reviewers are needed.

---

## 6. Absolute Prohibitions (10 Red Lines)

The following will result in **immediate rejection** regardless of any other factors. These are non-negotiable.

| # | Prohibition | Why |
|---|------------|-----|
| 1 | **Hardcoded private keys, mnemonics, or API secrets** | Credentials in source code are permanently exposed in version history. |
| 2 | **Command injection (`curl \| sh` with remote URLs)** | Remote scripts can be replaced at any time, enabling arbitrary code execution. |
| 3 | **Prompt injection attempts** | Instructions that override agent safety guardrails compromise all users. |
| 4 | **Credential exfiltration** | Any mechanism that sends local credentials (env vars, files) to external servers. |
| 5 | **Obfuscated code (base64 payloads, unicode tricks)** | Code that cannot be read by reviewers cannot be trusted. |
| 6 | **Persistence mechanisms (cron, launchctl, systemd)** | Background services survive plugin uninstall and can act as long-term backdoors. |
| 7 | **Accessing sensitive files (~/.ssh, ~/.aws, ~/.kube, ~/.gnupg)** | No plugin has a legitimate reason to read SSH keys or cloud credentials. |
| 8 | **Direct financial operations bypassing OnchainOS without declaration** | All on-chain write operations must go through the onchainos CLI. Self-implementing wallet signing, transaction broadcasting, or swap execution is forbidden. |
| 9 | **Supply chain attacks (unpinned dependencies + dynamic install)** | Runtime installation of unversioned packages opens an ever-present poisoning window. |
| 10 | **Memory poisoning attempts** | Writing to agent memory files (MEMORY.md, SOUL.md) to plant persistent cross-session instructions. |

---

## 7. Pre-Submission Checklist

Copy this checklist into your PR description before submitting:

```markdown
## Pre-Submission Checklist

- [ ] `plugin.yaml` exists and contains valid YAML
- [ ] `SKILL.md` exists with correct version matching `plugin.yaml`
- [ ] Category is one of: trading, defi, game, prediction, data_tools, dev_tools, others
- [ ] License field contains a valid SPDX identifier
- [ ] Safety defaults set: PAUSED=True, PAPER_TRADE=True, DRY_RUN=True
- [ ] No hardcoded secrets, private keys, or mnemonics in any file
- [ ] No `curl | sh` or `wget | sh` patterns
- [ ] No obfuscated code (base64 payloads, unicode encoding)
- [ ] No access to sensitive paths (~/.ssh, ~/.aws, ~/.kube)
- [ ] All on-chain write operations use onchainos CLI (no raw ethers.js, web3.py, etc.)
- [ ] All external URLs are reachable
- [ ] All package dependencies are version-pinned
- [ ] External data has untrusted-data boundary declaration in SKILL.md
- [ ] Financial operations include explicit user confirmation steps
- [ ] Python files pass syntax check
- [ ] Build succeeds on all target platforms (if applicable)
```

---

## 8. Appeals Process

If you believe a review decision is incorrect:

1. **Comment on the PR** with a clear explanation of why you disagree with the finding. Include evidence (code references, documentation links) supporting your case.
2. **A reviewer will respond within 2 business days** with either a revised decision or an explanation of why the original finding stands.
3. **Escalation**: If you are not satisfied with the response, open a GitHub Issue in the plugin-store repository with the title `[Appeal] <plugin-name> - <brief description>`. The issue will be reviewed by a senior maintainer.

Appeals are taken seriously. Automated rules include false-positive filtering, but edge cases exist. If a static rule flagged a placeholder value (e.g., `0xYourPrivateKeyHere`) or a documentation example rather than real code, provide that context in your appeal and it will be resolved quickly.
