# lista-lending-onchainos

## Overview

Lista DAO is a decentralized stablecoin and lending protocol on BNB Chain that issues lisUSD (a CDP-backed stablecoin) and operates a lending market powered by the Moolah lending SDK. This plugin wraps the Lista DAO SDK to build `pending_sign` lending transactions, routed through Onchain OS for TEE-based signing.

Core operations:
- List markets and vaults with current APY/APR
- Get user holdings and vault positions
- Build deposit, withdraw, supply, borrow, repay, and market-withdraw transactions
- Simulate borrow and repay operations before executing

Tags: `lending` `borrowing` `lista` `lista-dao` `bsc` `stablecoin`

## Prerequisites

- No IP restrictions
- Supported chains: BNB Chain (56), Ethereum (1)
- Supported tokens: USDC, USDT, WBNB, lisUSD, and other Lista-supported assets
- onchainos CLI installed and authenticated
- Node.js ≥ 18 and `tsx` for running the skill (`npm install` in skill directory)

## Quick Start

1. **Install dependencies**: `cd ~/.agents/skills/lista-lending-onchainos && npm install`
2. **List markets**: Ask Claude "show me Lista DAO lending markets on BSC"
3. **Check positions**: Ask Claude "show my Lista DAO holdings"
4. **Deposit collateral**: Claude will call `buildDeposit`, return a `pending_sign` envelope, and route it through `onchainos wallet contract-call` for signing and broadcast
