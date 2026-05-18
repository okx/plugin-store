---
title: Order Monitoring and Fulfillment Tracking
impact: HIGH
impactDescription: "Required for tracking cross-chain order status after execution"
tags: monitoring, order-status, dln, fulfillment, polling
---

# Order Monitoring

After broadcasting a bridge transaction, track the DLN order until fulfillment on the destination chain.

## Get Order ID

The order ID comes from the `create_tx` response (`orderId` field). Record it before signing.

If the order ID is lost, it can be recovered from the transaction hash using the deBridge Explorer.

## Poll Order Status

Query the DLN statistics API:

```
GET https://stats-api.dln.trade/api/Orders/{orderId}
```

### Example

```bash
curl -s "https://stats-api.dln.trade/api/Orders/$ORDER_ID" | jq '.status'
```

```typescript
const response = await fetch(`https://stats-api.dln.trade/api/Orders/${orderId}`);
const order = await response.json();
console.log("Status:", order.status);
```

## Status Values

| Status          | Meaning                                              | Action             |
|-----------------|------------------------------------------------------|--------------------|
| `None`          | Order not yet indexed                                | Wait, poll again   |
| `Created`       | Order submitted on source chain, awaiting fulfillment| Wait, poll again   |
| `Fulfilled`     | Order filled on destination chain by a taker         | Almost done        |
| `SentUnlock`    | Unlock transaction sent on source chain              | Almost done        |
| `ClaimedUnlock` | Fully completed — funds delivered, collateral unlocked | Done              |
| `Cancelled`     | Order was cancelled                                  | Check why          |

Normal flow: `Created` → `Fulfilled` → `SentUnlock` → `ClaimedUnlock`

## Polling Strategy

- **First poll**: 10 seconds after broadcast (order needs to be indexed).
- **Interval**: every 15–30 seconds.
- **Timeout**: most orders fulfill within 1–5 minutes. Alert the user if still `Created` after 10 minutes.
- **Terminal states**: `ClaimedUnlock` (success) or `Cancelled` (failure).

## deBridge Explorer

For manual inspection, the order can be viewed at:

```
https://app.debridge.com/order?orderId={orderId}
```

Share this link with the user for visual tracking.

## Common Issues

| Issue                        | Cause                          | Fix                                  |
|------------------------------|--------------------------------|--------------------------------------|
| Order stuck on `Created`     | No taker picked up the order   | Wait — takers may need more time     |
| Order `Cancelled`            | Expired or manually cancelled  | Re-create the bridge with `create_tx`|
| Order not found (404)        | Not yet indexed                | Wait 30 seconds and retry            |

## Post-Delivery Verification

After an order reaches `ClaimedUnlock`, verify the balance on the destination chain using:
- ../analytics/onchain-explorer.md — verify the transaction and token balances on the destination chain via Blockscout
