# Plugin Design: Raydium AMM

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `raydium` |
| `dapp_name` | Raydium AMM |
| `target_chains` | `[501]` (Solana mainnet) |
| `target_protocols` | AMM v4 (Legacy Standard), CPMM, CLMM (Concentrated Liquidity) |
| `plugin_type` | Swap / DEX |
| `onchainos_broadcast` | Yes |

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 社区维护的非官方 Rust crate 存在，但无官方支持，功能不完整，不推荐用于生产 |
| SDK 支持哪些技术栈？ | 官方：TypeScript (`@raydium-io/raydium-sdk-v2`)；社区：Rust（非官方） |
| 有 REST API？ | **Yes** — 两套 REST API：(1) 数据查询 `https://api-v3.raydium.io`；(2) 交易构建 `https://transaction-v1.raydium.io`。详见 https://docs.raydium.io/raydium/api-reference/overview |
| 有官方 Skill？ | No |
| 开源社区有类似 Skill？ | **Yes** — `kukapay/raydium-launchlab-mcp` (https://github.com/kukapay/raydium-launchlab-mcp)，但仅覆盖 LaunchLab（代币发行），未覆盖 AMM swap/pool 操作 |
| 支持哪些链？ | Solana mainnet only（chain ID 501） |
| 是否需要 onchainos 广播？ | **Yes** — swap 等写操作通过 `transaction-v1.raydium.io` 获取 base64 序列化交易，再经 `onchainos wallet contract-call --chain 501 --unsigned-tx <base64>` 或 `onchainos dex swap execute` 广播 |

### 接入路径

**路径：API**（Rust 调 Raydium REST API）

原因：
- 官方 SDK 仅有 TypeScript，无 Rust
- 社区 Rust SDK 不完整，不可靠
- Raydium 提供完整的 REST API（数据查询 + 交易构建），功能完备
- 交易构建 API 直接返回 base64 序列化交易，与 onchainos Solana 广播路径完美契合

---

## §2 操作接口映射

### 2a. 操作总表

| 操作名 | 类型 | 描述 |
|--------|------|------|
| `get-price` | 链下查询 | 查询指定代币对的当前价格及滑点估算 |
| `get-pools` | 链下查询 | 按代币 mint 地址或 pool ID 查询流动性池信息 |
| `get-pool-list` | 链下查询 | 分页获取所有流动性池列表，支持按类型和排序筛选 |
| `swap` | 链上写操作 | 执行代币兑换（AMM v4 / CPMM / CLMM 路由） |
| `get-swap-quote` | 链下查询 | 获取 swap 报价，含预期输出量、价格影响、路由 |
| `get-token-price` | 链下查询 | 查询指定 mint 地址代币的 USD 价格 |

### 2b. 链下查询接口

#### get-swap-quote
获取 swap 报价（仅计算，不构建交易）

- **Endpoint:** `GET https://transaction-v1.raydium.io/compute/swap-base-in`
- **参数：**
  | 参数名 | 类型 | 必填 | 说明 |
  |--------|------|------|------|
  | `inputMint` | string | Yes | 输入代币 mint 地址 |
  | `outputMint` | string | Yes | 输出代币 mint 地址 |
  | `amount` | u64 | Yes | 输入数量（基本单位，含 decimals） |
  | `slippageBps` | u32 | Yes | 滑点容忍度（基点，50 = 0.5%） |
  | `txVersion` | string | Yes | `"V0"` 或 `"LEGACY"` |
- **返回关键字段：**
  | 字段 | 类型 | 说明 |
  |------|------|------|
  | `data.outputAmount` | string | 预期输出数量（基本单位） |
  | `data.priceImpactPct` | f64 | 价格影响百分比 |
  | `data.routePlan` | array | 路由方案（含 pool IDs） |
  | `data.inputAmount` | string | 实际输入数量 |

#### get-price（通过 compute/swap-base-in 计算）

- **Endpoint:** `GET https://transaction-v1.raydium.io/compute/swap-base-in`
- **说明：** 用 `amount=1_000_000`（对于 6 decimals 代币为 1 单位）调用报价接口，从 `outputAmount / inputAmount` 计算价格比率
- **参数：** 同 `get-swap-quote`

#### get-token-price
查询代币 USD 价格

- **Endpoint:** `GET https://api-v3.raydium.io/mint/price`
- **参数：**
  | 参数名 | 类型 | 必填 | 说明 |
  |--------|------|------|------|
  | `mints` | string | Yes | 逗号分隔的 mint 地址列表 |
- **返回关键字段：**
  | 字段 | 类型 | 说明 |
  |------|------|------|
  | `data.<mint_address>` | f64 | 该 mint 的 USD 价格 |

#### get-pools
按 mint 地址或 pool ID 查询流动性池信息

- **Endpoint（按 ID）:** `GET https://api-v3.raydium.io/pools/info/ids`
  - 参数：`ids` — 逗号分隔的 pool ID 列表
- **Endpoint（按 mint）:** `GET https://api-v3.raydium.io/pools/info/mint`
  - 参数：`mint1`（必填，代币 mint 地址），`mint2`（可选），`poolType`，`poolSortField`，`sortType`，`pageSize`，`page`
- **返回关键字段（每个 pool）：**
  | 字段 | 类型 | 说明 |
  |------|------|------|
  | `id` | string | Pool ID |
  | `type` | string | `"Standard"` / `"Concentrated"` / `"CPMM"` |
  | `programId` | string | Solana 程序地址 |
  | `mintA.address` | string | Token A mint 地址 |
  | `mintB.address` | string | Token B mint 地址 |
  | `price` | f64 | Token A 对 Token B 当前价格 |
  | `tvl` | f64 | 总锁仓价值（USD） |
  | `feeRate` | f64 | 手续费率（如 0.25% = 0.0025） |
  | `lpAmount` | f64 | LP 代币总量 |

#### get-pool-list
分页获取流动性池列表

- **Endpoint:** `GET https://api-v3.raydium.io/pools/info/list`
- **参数：**
  | 参数名 | 类型 | 必填 | 说明 |
  |--------|------|------|------|
  | `poolType` | string | Yes | `all`/`concentrated`/`standard`/`allFarm` 等 |
  | `poolSortField` | string | Yes | `default`/`liquidity`/`volume24h`/`apr24h` 等 |
  | `sortType` | string | Yes | `desc` 或 `asc` |
  | `pageSize` | u32 | Yes | 每页数量（最大 1000） |
  | `page` | u32 | Yes | 页码（从 1 开始） |
- **返回：** pool 对象数组 + `hasNextPage` 布尔值

### 2c. 链上写操作

#### swap（代币兑换）

Solana 上 Raydium swap 分两步：
1. **Step 1（链下）** — 调用 `compute/swap-base-in` 获取报价
2. **Step 2（链下）** — 调用 `transaction/swap-base-in` 构建序列化交易
3. **Step 3（链上）** — 通过 onchainos 广播序列化交易

**Step 2 — 构建交易**

- **Endpoint:** `POST https://transaction-v1.raydium.io/transaction/swap-base-in`
- **Request Body：**
  ```json
  {
    "swapResponse": "<完整的 compute/swap-base-in 响应 JSON>",
    "txVersion": "V0",
    "wallet": "<用户 Solana 公钥>",
    "wrapSol": true,
    "unwrapSol": true,
    "computeUnitPriceMicroLamports": "auto"
  }
  ```
  | 字段 | 类型 | 必填 | 说明 |
  |------|------|------|------|
  | `swapResponse` | object | Yes | Step 1 报价接口的完整响应 |
  | `txVersion` | string | Yes | `"V0"` 推荐；`"LEGACY"` 备选 |
  | `wallet` | string | Yes | 用户的 Solana 公钥（base58） |
  | `wrapSol` | bool | No | 输入为原生 SOL 时设为 true（自动 wrapSOL） |
  | `unwrapSol` | bool | No | 输出为 WSOL 时设为 true（自动 unwrapSOL） |
  | `inputAccount` | string | No | 输入代币 ATA 地址（可选，不传则自动推导） |
  | `outputAccount` | string | No | 输出代币 ATA 地址（可选，不传则自动推导） |
  | `computeUnitPriceMicroLamports` | string/u64 | No | Priority fee，推荐 `"auto"` |
- **返回关键字段：**
  | 字段 | 类型 | 说明 |
  |------|------|------|
  | `data[].transaction` | string | base64 编码的序列化交易（可能返回多笔） |

**Step 3 — onchainos 广播**

```bash
# 方法 1：直接通过 wallet contract-call 广播 Solana 序列化交易
onchainos wallet contract-call \
  --chain 501 \
  --unsigned-tx <base64_serialized_transaction>

# 方法 2：通过 dex swap execute（onchainos 内部路由，推荐用于标准 token swap）
onchainos dex swap execute \
  --chain 501 \
  --from-token <input_mint_address> \
  --to-token <output_mint_address> \
  --readable-amount <human_amount>
```

> **重要提示（Solana tx 过期）：** Solana blockhash 约 60 秒过期。获取 `transaction-v1.raydium.io` 的 serializedTransaction 后，**必须立即**调用 onchainos 广播，不可缓存或延迟。

**txHash 提取：**
```
result["data"]["txHash"]
```

**相关程序地址（运行时动态验证，不硬编码）：**

| 程序 | 地址（仅供参考，需从 pools/info API 动态读取 programId） |
|------|------|
| AMM v4 (Legacy Standard) | `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` |
| CPMM | `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C` |
| CLMM (Concentrated) | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` |
| AMM Routing | `routeUGWgWzqBWFcrCfv8tritsqukccJPu3q5GPP3xS` |

> Raydium API 返回的每个 pool 对象包含 `programId`，交易构建时使用 API 返回值，不要硬编码。

---

## §3 用户场景

### 场景 1：查询 SOL/USDC 价格并执行 swap（Happy Path）

**用户说：** "帮我把 10 SOL 换成 USDC，在 Raydium 上"

**Agent 动作序列：**

1. **[链下查询]** 解析用户意图：输入 = SOL，输出 = USDC，数量 = 10
2. **[链下查询]** 调用 `onchainos token search --query SOL --chain 501` 和 `onchainos token search --query USDC --chain 501` 确认 mint 地址
   - SOL（原生）: `So11111111111111111111111111111111111111112`（WSOL）
   - USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
3. **[链下查询]** 安全检查：`onchainos security token-scan --address EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --chain 501`
4. **[链下查询]** 查询当前 SOL 余额：`onchainos wallet balance --chain 501`，确认余额 ≥ 10 SOL（+ gas）
5. **[链下查询]** 调用 `GET https://transaction-v1.raydium.io/compute/swap-base-in` 获取报价
   - `inputMint=So11111111111111111111111111111111111111112`
   - `outputMint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
   - `amount=10000000000`（10 SOL × 10^9）
   - `slippageBps=50`（0.5%）
   - `txVersion=V0`
6. **[展示给用户]** 显示报价：预期获得约 XXX USDC，价格影响 Y%，手续费 Z SOL
7. **[等待确认]** 如价格影响 >5% 则发出警告；等待用户确认
8. **[链下]** 调用 `POST https://transaction-v1.raydium.io/transaction/swap-base-in`，body 含 Step 5 报价响应、wallet 地址、`wrapSol: true`
9. **[链上操作]** 立即广播：`onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>`
10. **[验证]** 提取 `result["data"]["txHash"]`，通过 `onchainos wallet history --tx-hash <hash>` 确认成功

---

### 场景 2：查询流动性池信息（纯查询场景）

**用户说：** "Raydium 上 RAY/USDC 池子的 TVL 和年化是多少？"

**Agent 动作序列：**

1. **[链下查询]** 解析：查询 RAY/USDC 池信息
2. **[链下查询]** 获取 RAY mint 地址：`onchainos token search --query RAY --chain 501`
   - RAY: `4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R`
3. **[链下查询]** 调用 `GET https://api-v3.raydium.io/pools/info/mint`
   - `mint1=4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R`
   - `mint2=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
   - `poolType=all`，`poolSortField=liquidity`，`sortType=desc`，`pageSize=5`，`page=1`
4. **[展示给用户]** 返回各类型池（Standard/CPMM/CLMM）的：
   - TVL（USD）
   - 24h/7d/30d 手续费 APR
   - 当前价格
   - 手续费率

---

### 场景 3：包含风控的 swap（小流动性代币，高价格影响）

**用户说：** "帮我用 1000 USDC 买一个新的 memecoin，mint 地址是 AbcXXX..."

**Agent 动作序列：**

1. **[风控]** 安全扫描：`onchainos security token-scan --address AbcXXX... --chain 501`
   - 如果 `action: "block"`（蜜罐）→ **拒绝执行**，告知用户风险
   - 如果代币上线 <24h → 发出警告
2. **[链下查询]** 查询代币流动性：`onchainos token liquidity --address AbcXXX... --chain 501`
   - 如果流动性 <$10K → 发出高滑点警告
3. **[链下查询]** 调用 `GET https://transaction-v1.raydium.io/compute/swap-base-in` 获取报价
   - `amount=1000000000`（1000 USDC，6 decimals），`slippageBps=300`（3%，考虑低流动性）
4. **[风控]** 检查 `priceImpactPct`：
   - 如 >5% → 发出警告，要求用户二次确认
   - 如 >20% → 建议分批执行或放弃
5. **[等待确认]** 展示完整风险提示，等待用户明确确认
6. **[链下]** 构建交易：`POST https://transaction-v1.raydium.io/transaction/swap-base-in`
7. **[链上操作]** 立即广播：`onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>`
8. **[验证]** 确认 txHash 并告知用户交易结果

---

### 场景 4：查询代币实时价格

**用户说：** "Raydium 上 SOL 现在多少钱？"

**Agent 动作序列：**

1. **[链下查询]** 调用 `GET https://api-v3.raydium.io/mint/price?mints=So11111111111111111111111111111111111111112`
2. **[展示]** 返回 SOL 的 USD 价格
3. **[可选]** 同时调用 `onchainos market price --address So11111111111111111111111111111111111111112 --chain 501` 交叉验证

---

## §4 外部 API 依赖

| API | Base URL | 用途 | 认证 |
|-----|----------|------|------|
| Raydium Data API v3 | `https://api-v3.raydium.io` | 池子信息、代币价格查询（只读） | 无（公开） |
| Raydium Transaction API v1 | `https://transaction-v1.raydium.io` | swap 报价计算、序列化交易构建 | 无（公开） |
| onchainos CLI | 本地 CLI | 钱包地址解析、链上广播、代币搜索、安全检查 | 需已登录（`wallet status`） |

### 关键 Endpoint 汇总

| 操作 | 方法 | URL |
|------|------|-----|
| 获取代币 USD 价格 | GET | `https://api-v3.raydium.io/mint/price?mints=<mints>` |
| 按 Pool ID 查询池子 | GET | `https://api-v3.raydium.io/pools/info/ids?ids=<ids>` |
| 按 mint 地址查询池子 | GET | `https://api-v3.raydium.io/pools/info/mint?mint1=<m1>&mint2=<m2>&...` |
| 分页获取池子列表 | GET | `https://api-v3.raydium.io/pools/info/list?poolType=all&...` |
| 获取 swap 报价 | GET | `https://transaction-v1.raydium.io/compute/swap-base-in` |
| 构建 swap 序列化交易 | POST | `https://transaction-v1.raydium.io/transaction/swap-base-in` |

---

## §5 配置参数

| 参数名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `chain_id` | u64 | `501` | Solana mainnet chain ID（固定） |
| `slippage_bps` | u32 | `50` | 默认滑点（基点，50 = 0.5%） |
| `tx_version` | string | `"V0"` | Solana tx 版本，`"V0"` 或 `"LEGACY"` |
| `wrap_sol` | bool | `true` | 输入为原生 SOL 时是否自动 wrap |
| `unwrap_sol` | bool | `true` | 输出为 WSOL 时是否自动 unwrap |
| `compute_unit_price` | string | `"auto"` | Priority fee（`"auto"` 或微 lamports 数值字符串） |
| `price_impact_warn_pct` | f64 | `5.0` | 价格影响超过此阈值时警告用户 |
| `price_impact_block_pct` | f64 | `20.0` | 价格影响超过此阈值时建议放弃 |
| `min_liquidity_usd` | f64 | `10000.0` | 低于此 TVL 时发出高滑点风险警告 |
| `data_api_base_url` | string | `"https://api-v3.raydium.io"` | Raydium 数据 API base URL |
| `tx_api_base_url` | string | `"https://transaction-v1.raydium.io"` | Raydium 交易构建 API base URL |
| `dry_run` | bool | `false` | dry_run=true 时跳过 onchainos 广播，返回模拟响应 |
| `mev_protection` | bool | `true` | SOL swap 金额 >$1000 时启用 MEV 保护（`--mev-protection`） |

---

## 附录：Solana Swap 完整流程图

```
用户请求 swap(inputMint, outputMint, amount, slippageBps)
  │
  ├─ [1] onchainos token search → 确认 mint 地址
  ├─ [2] onchainos security token-scan → 安全检查
  ├─ [3] onchainos wallet balance → 余额检查
  ├─ [4] GET /compute/swap-base-in → 获取报价 (quoteResponse)
  ├─ [5] 展示报价 + 风险提示，等待用户确认
  ├─ [6] POST /transaction/swap-base-in (body: {swapResponse: quoteResponse, wallet, txVersion, wrapSol, unwrapSol})
  │       → 返回 data[].transaction (base64 serialized tx)
  └─ [7] 立即（<60s）执行:
         onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>
         → result["data"]["txHash"]
```

> **注意：** Solana blockhash 约 60 秒过期。Step 6 和 Step 7 必须连续执行，不可等待用户二次确认（确认应在 Step 5 完成）。
