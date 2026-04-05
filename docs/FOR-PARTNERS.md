# For Partners

This guide is for OKX internal teams and external partners who want to publish plugins to the OKX Plugin Store. It covers submission flows, trust badges, visibility options, and -- most critically -- the high-risk plugin policy.

---

## 1. Partner Types

| Type | Submit To | Trust Badge | Example |
|------|-----------|-------------|---------|
| OKX Internal (low risk) | `skills/` via OKX org account | Official | DEX query tools, price feeds, portfolio viewers |
| OKX Internal (high risk) | `skills/` via personal GitHub account | Community | Auto-trading strategies, sniping bots, copy-trade systems |
| External Partner | `skills/` via partner account | Verified Partner | Uniswap, Polymarket, third-party DeFi protocols |

**Badge meanings:**
- **Official** -- Built and maintained by OKX. Displayed prominently with OKX branding.
- **Community** -- Independent contribution. No OKX brand association.
- **Verified Partner** -- Built by a vetted external partner. Displayed with partner branding.

---

## 2. High-Risk Plugin Policy

> **This is the most important section in this document. Read it carefully.**

### Core Rule

**The OKX brand must NOT be associated with high-risk trading strategies.**

High-risk plugins developed internally must be submitted under a personal GitHub account and presented as community contributions -- never as official OKX products.

### What Qualifies as High Risk

A plugin is classified as high-risk if it meets **any one** of the following criteria:

- Automated trading without per-trade user confirmation
- Copy trading or signal following
- Sniping or MEV strategies
- Potential for large single-trade amounts without explicit limits
- Complex strategy logic that users may not fully understand

### Handling Rules

| Scenario | Submit As | Badge | Marketing Language |
|----------|-----------|-------|--------------------|
| OKX internal, low-risk plugin | OKX org account to `skills/` | Official | "OKX Official" branding permitted |
| OKX internal, high-risk plugin | Personal GitHub account to `skills/` | Community | "Community showcase" only. **No OKX branding whatsoever.** |
| External partner plugin | Partner account to `skills/` | Verified Partner | Joint branding permitted. Must include "by [Partner Name]". |

### Correct Example

A team inside OKX builds an automated trading bot. They submit it under a personal GitHub account (e.g., `alice-dev`). The plugin listing says "Community showcase" and makes no mention of OKX. Users see the Community badge and understand this is an independent contribution.

### Wrong Example

The same team submits the trading bot under the `okx` org account. The plugin listing says "OKX Official Auto-Trading Bot." Users associate the risky strategy with the OKX brand. If the strategy causes losses, OKX bears reputational and potentially legal liability.

**This policy is non-negotiable.** PRs that violate this rule will be rejected regardless of code quality.

---

## 3. Internal Team Submission Flow

For OKX employees and internal teams:

1. **Create a branch** following the naming convention:
   ```
   partner/<team-name>/<plugin-name>
   ```
   Example: `partner/dex-team/swap-aggregator`

2. **Add your plugin** to the `skills/<plugin-name>/` directory with all required files (`plugin.yaml`, `SKILL.md`, source code if applicable).

3. **Submit a PR** using the standard PR template. Fill in the pre-submission checklist (see [REVIEW-GUIDELINES.md](./REVIEW-GUIDELINES.md)).

4. **Automated review** runs through all four phases (Lint, Build, AI Review, Human Review). Internal submissions go through the same automated pipeline but may receive expedited human review (internal fast track).

5. **Merge and publish.** Once approved, the plugin appears in the Plugin Store registry.

**Reminder:** If your plugin is high-risk, submit from a personal GitHub account, not the OKX org account. See Section 2.

---

## 4. External Partner Submission Flow

For companies and projects outside OKX:

1. **Contact the OKX BD (Business Development) team** to express interest in publishing a plugin. Provide your company name, plugin concept, and target chains.

2. **Technical assessment.** The BD team connects you with the Plugin Store engineering team for a technical feasibility review.

3. **Sign a partnership agreement** covering plugin maintenance responsibilities, incident response obligations, and branding guidelines.

4. **Receive Verified Partner access.** You will be granted a GitHub account or team with write access to submit PRs to the plugin-store repository.

5. **Submit your plugin** to `skills/<plugin-name>/` following the standard structure and PR template.

6. **Full review.** Your submission goes through the complete 4-phase review pipeline. The partner channel provides a dedicated reviewer for questions during the process.

7. **Merge and publish.** Once approved, the plugin appears in the registry with the Verified Partner badge and your branding.

---

## 5. Showcase and Visibility

| Placement | Eligibility | How to Get |
|-----------|-------------|------------|
| README featured table | Official and Verified Partner plugins | Automatically included upon merge |
| Category top spot | Best plugin in a given category | Selected by the Plugin Store PM |
| FOR-USERS.md case study | Any plugin with a compelling usage example | Submit a 3-line usage example with your PR |
| Website featured | PM-approved plugins with broad appeal | Contact the Plugin Store PM directly |

Visibility placements are reviewed quarterly. High-quality plugins with active maintenance and good user feedback are prioritized.

---

## 6. Information Required

### Internal Team Submission Form

Provide the following information in your PR description or accompanying document:

| Field | Description |
|-------|-------------|
| Plugin name | Short, descriptive name (lowercase, hyphens allowed) |
| Description | One-paragraph summary of what the plugin does |
| Category | One of: `trading`, `defi`, `game`, `prediction`, `data_tools`, `dev_tools`, `others` |
| Risk level | `low`, `medium`, or `high` (see Section 2 for classification) |
| Strategy overview | Internal-only description of the strategy logic (will not be published) |
| Usage examples | Three example commands or workflows showing the plugin in action |
| Target date | Planned launch date |
| Submission method | OKX org account (low-risk only) or personal account (high-risk) |
| GitHub account | The GitHub username that will own the submission |

### External Partner Submission Form

| Field | Description |
|-------|-------------|
| Company name | Legal entity name |
| Contact | Name and email of the primary technical contact |
| Plugin description | Detailed description of what the plugin does and its value to users |
| Supported chains | List of blockchains the plugin interacts with |
| API docs link | URL to your API documentation (if the plugin calls your APIs) |
| Brand assets | Logo (SVG preferred) and tagline for marketplace listing |
| Target launch date | Planned launch date |

---

## 7. Incident Response

If a published plugin is found to have a security vulnerability, malicious behavior, or a critical bug, the following timeline applies:

| Timeframe | Action |
|-----------|--------|
| **Immediate** | Mark the plugin as `suspended` in `registry.json`. Users are warned not to install or use it. |
| **Within 1 hour** | Merge a PR to disable installation. Existing installs are flagged in the CLI with a security warning. |
| **Within 24 hours** | Complete a root cause analysis. The plugin author (internal team or external partner) is notified and provided with findings. |
| **Follow-up** | The author submits a fix and the plugin goes through the full review pipeline again. If the issue cannot be resolved, the plugin is permanently removed from the registry. |

### Partner Responsibilities During Incidents

- **Internal teams**: Respond to the incident channel within 1 hour during business hours. Provide a fix or mitigation plan within 24 hours.
- **External partners**: Respond within 4 hours during business hours (per partnership agreement). Provide a fix within 48 hours or the plugin will be permanently removed.

Repeated incidents (3 or more within 6 months) may result in revocation of publishing privileges.
