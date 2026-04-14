# Security Policy

## Reporting

If you discover a security issue, please report it privately to the maintainer instead of opening a public issue with exploit details.

## Scope notes

BPB Lite is a public research and triage skill.

It does not ship private alert-routing logic, private trading infrastructure, or production secrets from the full Based Pings system.

## Secret handling

- never commit API tokens
- keep optional attribution tokens in local env files only
- avoid sharing logs that expose secrets or privileged endpoints

## Public repo safety

Contributions that expose private operational logic, unpublished production thresholds, or live sensitive infrastructure details will be rejected.
