# Security Checklist

Use this checklist when reviewing or building Across integrations.

## Integration Security

### Token Handling
- [ ] Use correct token addresses for each specific chain. Do not hardcode cross-chain.
- [ ] Use wrapped token addresses (WETH) when interacting with APIs, not native ETH.
- [ ] Validate that `amount` is in the correct decimals for the token (6 for USDC, 18 for WETH).
- [ ] Never trust user-supplied token addresses without validating against `/available-routes` or `/swap/tokens`.

### Approvals
- [ ] Process all `approvalTxns` from the Swap API response before submitting the swap tx.
- [ ] Do not set unlimited approvals unless the user explicitly opts in. Prefer exact-amount approvals.
- [ ] Verify the spender address matches the contract returned by the API.

### Slippage and Pricing
- [ ] Use `slippage=auto` unless the user has a specific tolerance.
- [ ] If using numeric slippage, validate it is within a reasonable range (for example, 0.001 to 0.05).
- [ ] Do not cache quotes. `/swap/approval` and `/suggested-fees` are derived from on-chain state.
- [ ] Check `quoteExpiryTimestamp` before submitting a swap transaction. Fetch a new quote if expired.

### Refund Handling
- [ ] Understand default refund behavior when `refundOnOrigin` is not set:
  - B2B or A2B routes: refund on origin chain.
  - B2A or A2A routes: refund on destination chain.
- [ ] Set `refundAddress` explicitly if the depositor and intended refund recipient differ.
- [ ] Refund recipient priority is `refundAddress` > `recipient` > `depositor`.
- [ ] Handle the `expired` deposit status in your UI and inform users a refund is in progress.

### Embedded Actions (POST /swap/approval)
- [ ] Validate all `target` addresses in the actions array. Do not allow user-controlled targets without sanitization.
- [ ] For actions that require precise amounts, use static values and avoid `populateDynamically`.
- [ ] Actions execute in array order. Sequence dependencies explicitly.

### Message Construction (if using /suggested-fees with message)
- [ ] The `message` field is calldata passed to `handleAcrossMessage()` on the recipient contract.
- [ ] If `recipient` is an EOA and `message` is defined, the API will reject the request.
- [ ] Validate message encoding carefully. Malformed calldata will cause the destination call to revert.

### Integrator ID
- [ ] Register for a 2-byte hex integrator ID before going to production.
- [ ] Use a consistent `integratorId` across all API calls for tracking and support.

## Protocol-Level Security

### Verification Model
Across uses UMA's Optimistic Oracle for settlement verification. Bundles of fills are proposed on-chain with a bond and pass through a challenge period before execution. Invalid bundles can be disputed.

### Audits
The protocol has undergone multiple security audits:
- Across V3 Audit
- Across V3 Incremental Audit
- Across V2 Audit
- Across Token and Token Distributor Audit
- UMA Continuous Audit (Optimistic Oracle)
- UMA Audit - L2 Bridges
- UMA Audit - Phase 4

### Bug Bounty
Active bug bounty program covering all smart contracts and off-chain code in the `across-protocol` repository.

| Severity | Reward |
|----------|--------|
| Low | $250 |
| Medium | $1,000 |
| High | $10,000 |
| Critical | Up to $1,000,000 |

Severity follows OWASP risk rating (Impact x Likelihood).

Submit reports to: bugs@across.to

## Risk Notes Template
When implementing changes that touch signing, fees, slippage, token approvals, refund handling, or crosschain message construction, include a risk notes section:

```
## Risk Notes
- **Signing**: [what is being signed and by whom]
- **Fees**: [fee model used, who pays, integrator fee setup]
- **Slippage**: [tolerance setting and rationale]
- **Approvals**: [what tokens, what amounts, what spender]
- **Refunds**: [expected refund chain and recipient]
- **Messages**: [any crosschain calldata and its purpose]
```
