# Embedded Crosschain Actions

Embedded actions let you execute contract calls on the destination chain immediately after a bridge/swap using `POST /swap/approval`. This is useful for minting, staking, or depositing into a protocol as part of the same user flow.

## How it works
1. Submit `POST /swap/approval` with an `actions` array in the JSON body. All the same query parameters as `GET /swap/approval` apply (`tradeType`, `amount`, `inputToken`, `outputToken`, `originChainId`, `destinationChainId`, `depositor`, etc.).
2. The swap and bridge execute.
3. Actions execute on the destination chain in the order provided.

## Action Object Structure

```json
{
  "actions": [
    {
      "target": "0x...",
      "functionSignature": "function transfer(address,uint256)",
      "args": [
        {
          "value": "0xRecipientAddress",
          "populateDynamically": false
        },
        {
          "value": "0",
          "populateDynamically": true,
          "balanceSourceToken": "0xTokenAddress"
        }
      ],
      "value": "0",
      "isNativeTransfer": false
    }
  ]
}
```

## Field Reference

| Field | Description |
|-------|-------------|
| `target` | Target contract address for the action |
| `functionSignature` | Function signature to call (for example, `function transfer(address,uint256)`) |
| `args[]` | Array of function arguments |
| `args[].value` | Static value for the argument |
| `args[].populateDynamically` | If true, populate the argument at execution time |
| `args[].balanceSourceToken` | Token address to source the balance from (required when `populateDynamically` is true) |
| `value` | Native token value (wei) to send with the call |
| `isNativeTransfer` | If true, execute a native token transfer instead of a contract call |

## Dynamic Argument Population
When `populateDynamically` is true, the argument value is populated using the balance of `balanceSourceToken` at execution time. Use this when you want the action to consume the full swapped balance of a token.

## Common Patterns

### Transfer ERC-20 after swap
```json
{
  "target": "0xTokenAddress",
  "functionSignature": "function transfer(address to, uint256 value)",
  "args": [
    { "value": "0xRecipient", "populateDynamically": false },
    { "value": "0", "populateDynamically": true, "balanceSourceToken": "0xTokenAddress" }
  ],
  "value": "0",
  "isNativeTransfer": false
}
```

### Simple native transfer
```json
{
  "target": "0xRecipient",
  "functionSignature": "",
  "args": [],
  "value": "10000000000000000",
  "isNativeTransfer": true
}
```

## Failure Behavior
- If the bridge/swap succeeds but an action reverts, the bridged tokens remain in the multicall handler contract on the destination chain. They are **not lost** — the recipient can withdraw them manually.
- Actions are executed atomically in order. If action N reverts, actions N+1 onward are also skipped.
- Check the `actionsSucceeded` field in the `GET /deposit/status` response to determine whether actions executed successfully after a fill.

## Important Notes
- Actions execute in array order. Sequence dependencies explicitly.
- If you need an exact amount for a downstream action, use static values and avoid dynamic balance population.
- Use `tradeType=exactOutput` for workflows that require a precise destination amount.
- For simple "bridge and send to a different recipient" use cases, you do NOT need embedded actions — just set the `recipient` query parameter on `GET /swap/approval`.
