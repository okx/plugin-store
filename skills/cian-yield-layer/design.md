# CIAN Yield Layer — Plugin Store 接入 PRD

## 0. Plugin Meta

| Field            | Value                                           |
|------------------|-------------------------------------------------|
| plugin_name      | `cian-yield-layer`                              |
| dapp_name        | CIAN Yield Layer                                |
| record_id        | `recvfIVNIZnAoT`                                |
| category         | yield / staking                                 |
| tags             | yield, staking, ERC4626, delta-neutral, LST, LRT, ETH, BTC |
| target_chains    | Ethereum mainnet (chain_id 1)                   |
| target_protocols | CIAN Yield Layer                                |
| priority         | P1                                              |
| onchainos        | 是                                               |

---

## 1. Background

### 1.1 DApp 介绍

CIAN Yield Layer 是 DeFi 的收益层（Yield Layer），将跨链收益源整合、重组为结构化回报并分发给代币持有者。协议将用户资产存入 ERC4626 Vault，通过 Recursive Staking / Recursive Restaking / Hybrid Long-Short 等策略放大收益。

核心策略类型：
- **Recursive Staking**（RS）：递归质押，将 stETH / wstETH 存入 AAVE 反复借贷放大 LST 收益（最高 6.5x）
- **Recursive Restaking**（RR）：递归再质押，用 weETH / rsETH 等 LRT 借 ETH 再质押，放大再质押收益（最高 9x）
- **Hybrid Long-Short**：同时做多 LRT 收益 + 做空 ETH 的混合策略

主要部署链：**Ethereum Mainnet（主力链）**，另有 Arbitrum / Mantle 等跨链 Pool 作为分发层。

### 1.2 接入可行性调研

| 维度                 | 结论                                                                               |
|---------------------|-----------------------------------------------------------------------------------|
| 是否有 SDK            | 无公开 SDK                                                                          |
| 是否有 REST API       | 有（`yieldlayer.cian.app`，提供 Vault 列表、APY、TVL、用户持仓等只读端点）                |
| 合约是否开源          | 是（实现合约在 Etherscan 已验证，TransparentUpgradeableProxy 模式）                     |
| 合约是否经过审计      | 是（Ackee Blockchain 审计，无 Critical/High 发现）                                    |
| 主部署链              | Ethereum Mainnet（chain_id 1）                                                      |
| 接入路径              | 直接合约交互 + REST API 辅助只读查询                                                   |
| 取款是否有锁定期       | **是**，异步延迟赎回：用户先调用 `requestRedeem`，约 **5 天后** rebalancer 执行 `optionalRedeem` |
| 是否需要 Approve      | 是，存入 ERC-20（stETH、wstETH、weETH 等）前须 `approve` Vault 合约；直接存 ETH 不需要  |
| 关键风险              | 赎回依赖 rebalancer（链外实体）执行确认；存在 depeg 风险和杠杆清算风险                      |

### 1.3 接入路径

- **写操作**：直接合约交互，通过 `onchainos wallet contract-call` 广播
- **读操作**：优先使用 REST API（`yieldlayer.cian.app`）；余额类查询通过 ERC4626 标准函数 `eth_call`

---

## 2. DApp 核心能力 & 接口映射

### 2.1 核心 Vault 合约地址（Ethereum Mainnet）

| Vault 名称               | Proxy 地址（用户交互地址）                         | 收据代币符号   | 存入资产                        |
|-------------------------|--------------------------------------------------|------------|-------------------------------|
| stETH Yield Layer       | `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d`    | ylstETH    | ETH / WETH / stETH / wstETH / eETH / weETH |
| pumpBTC Yield Layer     | `0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b`    | ylpumpBTC  | pumpBTC / WBTC / FBTC         |

> **注意**：始终使用 Proxy 地址而非 Implementation 地址进行所有调用。
>
> - ylstETH Implementation（当前）：`0xa1dc0b6a02ab091580dc57bdd5fe8a9e577e0842`（VaultYieldETH）
> - ylpumpBTC Implementation（当前）：`0x7a9ca85e0d4f32004d47620df03982b1afd18e37`（VaultYieldBTC）
>
> 上述实现合约地址可能升级，始终通过 Proxy 调用。

### 2.2 关键 ERC-20 Token 地址（Ethereum Mainnet）

| Token   | 地址                                           | Decimals |
|---------|----------------------------------------------|---------|
| WETH    | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`| 18      |
| stETH   | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84`| 18      |
| wstETH  | `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0`| 18      |
| eETH    | `0x35fA164735182de50811E8e2E824cFb9B6118ac2`| 18      |
| weETH   | `0xCd5fE23C85820F7B72D0926FC9b05b43E359b7ee`| 18      |
| pumpBTC | `0xF469fBD2abcd6B9de8E169d128226C0Fc90a6Ff9`| 8       |
| WBTC    | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599`| 8       |

### 2.3 操作列表

---

#### 操作 1：查询 Vault 列表与 APY（REST API，只读）

**用途**：获取 Ethereum 上所有 CIAN Yield Layer Vault 的名称、地址、APY、TVL。

| 字段        | 值                                                              |
|-----------|----------------------------------------------------------------|
| 类型        | HTTP GET                                                        |
| Endpoint  | `https://yieldlayer.cian.app/ethereum/pool/home`               |
| 无需认证    | 公开端点                                                          |
| 返回格式    | JSON                                                            |

**响应示例**：
```json
{
  "code": "ok",
  "status": 0,
  "msg": "server response ok",
  "data": [
    {
      "chain_id": 1,
      "pool_name": "stETH Yield Layer",
      "pool_address": "0xB13aa2d0345b0439b064f26B82D8dCf3f508775d",
      "pool_type": "yield-layer",
      "apy": "22.5235",
      "apy_7": "22.5235",
      "apy_instant_7": "8.5235",
      "apy_esti_by_points_7": "4",
      "apy_eco_earn_7": "10",
      "tvl_usd": "392058499.97",
      "net_tvl_usd": "66076130.15"
    }
  ]
}
```

---

#### 操作 2：查询用户持仓（REST API，只读）

**用途**：查询指定用户在某 Vault 中的持仓数据（share 余额、资产价值、Points 积分等）。

| 字段        | 值                                                                                                  |
|-----------|-----------------------------------------------------------------------------------------------------|
| 类型        | HTTP GET                                                                                            |
| Endpoint  | `https://yieldlayer.cian.app/ethereum/pool/home/vault/user/{vault_address}?user_address={user_address}` |
| 参数        | `vault_address`：Vault Proxy 地址；`user_address`：用户钱包地址                                         |

---

#### 操作 3：查询 Vault 详情与收益分解（REST API，只读）

**用途**：获取指定 Vault 的详细 APY 分解（基础 APY、Points APY、Eco Earn APY）及策略分配情况。

| 字段        | 值                                                                                              |
|-----------|-----------------------------------------------------------------------------------------------|
| 类型        | HTTP GET                                                                                        |
| Endpoint  | `https://yieldlayer.cian.app/ethereum/pool/home/vault/breakdown/{vault_address}`              |

---

#### 操作 4：查询用户 Share 余额（链上只读）

**用途**：通过 ERC4626 标准接口查询用户持有的 Vault share token 数量（ylstETH 或 ylpumpBTC）。

| 字段                     | 值                                                             |
|------------------------|---------------------------------------------------------------|
| 合约                     | Vault Proxy（如 `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d`）|
| 函数签名（canonical）     | `balanceOf(address)`                                          |
| Selector               | `0x70a08231` ✅                                                |
| 参数                     | `(address account)`                                           |
| 返回值                   | `uint256` — 用户持有的 share 数量（18 decimals）                 |
| 调用方式                  | eth_call（只读）                                                |

**eth_call 示例**：
```bash
onchainos rpc eth_call \
  --chain 1 \
  --to 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d \
  --data 0x70a08231000000000000000000000000<user_address_padded_32bytes>
```

---

#### 操作 5：查询 Share 对应资产价值（链上只读）

**用途**：将用户 share 数量换算为对应的底层资产数量（如 WETH）。

| 字段                     | 值                                                             |
|------------------------|---------------------------------------------------------------|
| 合约                     | Vault Proxy                                                    |
| 函数签名（canonical）     | `convertToAssets(uint256)`                                    |
| Selector               | `0x07a2d13a` ✅                                                |
| 参数                     | `(uint256 shares)`                                            |
| 返回值                   | `uint256` — 对应的底层资产数量                                    |
| 调用方式                  | eth_call（只读）                                                |

---

#### 操作 6：查询 Vault 汇率（链上只读）

**用途**：获取当前 Vault 的 exchangePrice（每 share 对应的底层资产价值，18 decimals，初始值 1e18）。

| 字段                     | 值                                                             |
|------------------------|---------------------------------------------------------------|
| 合约                     | Vault Proxy                                                    |
| 函数签名（canonical）     | `exchangePrice()`                                             |
| Selector               | `0x9e65741e` ✅                                                |
| 参数                     | 无                                                             |
| 返回值                   | `uint256` — 汇率（1e18 精度）                                    |
| 调用方式                  | eth_call（只读）                                                |

---

#### 操作 7：查询 Vault 总资产（链上只读）

**用途**：查询 Vault 管理的底层资产总量（TVL）。

| 字段                     | 值                                                             |
|------------------------|---------------------------------------------------------------|
| 合约                     | Vault Proxy                                                    |
| 函数签名（canonical）     | `totalAssets()`                                               |
| Selector               | `0x01e1d114` ✅                                                |
| 参数                     | 无                                                             |
| 返回值                   | `uint256` — 总资产量（按底层资产精度）                              |
| 调用方式                  | eth_call（只读）                                                |

---

#### 操作 8：ERC-20 Approve（链上操作，存款前置步骤）

**用途**：授权 Vault 合约从用户地址转移 ERC-20 资产（stETH、wstETH、weETH、pumpBTC 等）。存入原生 ETH 时不需要此步骤。

| 字段                     | 值                                                             |
|------------------------|---------------------------------------------------------------|
| 合约                     | 被存入的 ERC-20 Token 合约                                       |
| 函数签名（canonical）     | `approve(address,uint256)`                                    |
| Selector               | `0x095ea7b3` ✅                                                |
| 参数                     | `(address spender, uint256 amount)`                           |
| spender                | Vault Proxy 地址                                               |
| amount                 | 存款数量（uint256，含 token decimals）；可设为 `type(uint256).max`  |
| 调用方式                  | onchainos wallet contract-call（需用户签名）                     |

**onchainos 命令示例（Approve stETH）**：
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 \
  --input-data 0x095ea7b3 \
               000000000000000000000000B13aa2d0345b0439b064f26B82D8dCf3f508775d \
               ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
```

---

#### 操作 9：存款 — optionalDeposit（链上操作，主要入口）

**用途**：将 ETH 或多种 LST/LRT token 存入 Vault，获取 share token（如 ylstETH）。支持 ETH（msg.value）、WETH、stETH、eETH、wstETH、weETH（针对 ylstETH Vault）。存 ERC-20 前须先 Approve（操作 8）。

| 字段                     | 值                                                                                                      |
|------------------------|--------------------------------------------------------------------------------------------------------|
| 合约                     | Vault Proxy（如 `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d`）                                         |
| 函数签名（canonical）     | `optionalDeposit(address,uint256,address,address)`                                                     |
| Selector               | `0x32507a5f` ✅                                                                                         |
| 参数                     | `(address _token, uint256 _assets, address _receiver, address _referral)`                              |
| `_token`               | 存入 token 地址（存原生 ETH 时传 `address(0)` 或零地址，并附 msg.value）                                    |
| `_assets`              | 存款数量（uint256，含 token decimals）；存 ETH 时传 `0`，金额由 msg.value 决定                               |
| `_receiver`            | 接收 share token 的地址（通常为 msg.sender）                                                              |
| `_referral`            | 推荐人地址（无推荐时传 `address(0)`）                                                                       |
| 返回值                   | `uint256 shares_` — 铸造的 share 数量                                                                   |
| msg.value              | 存原生 ETH 时需附 ETH 金额（wei）；存 ERC-20 时为 0                                                         |
| 调用方式                  | onchainos wallet contract-call（payable，存 ETH 时附 --value）                                          |

**onchainos 命令示例（存入 0.1 stETH）**：
```bash
# 前置：approve stETH → Vault（操作 8）
# 存款：optionalDeposit(stETH, 0.1e18, receiver, address(0))
onchainos wallet contract-call \
  --chain 1 \
  --to 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d \
  --input-data <abi_encoded_calldata>
# calldata = 0x32507a5f + abi.encode(stETH_addr, 100000000000000000, receiver_addr, address(0))
```

**onchainos 命令示例（存入 0.1 ETH）**：
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d \
  --value 100000000000000000 \
  --input-data <abi_encoded_calldata>
# calldata = 0x32507a5f + abi.encode(address(0), 0, receiver_addr, address(0))
```

---

#### 操作 10：存款 — deposit（ERC4626 标准，链上操作）

**用途**：ERC4626 标准存款接口，仅接受 Vault 底层资产（WETH for ylstETH；pumpBTC for ylpumpBTC）。

| 字段                     | 值                                                             |
|------------------------|---------------------------------------------------------------|
| 合约                     | Vault Proxy                                                    |
| 函数签名（canonical）     | `deposit(uint256,address)`                                    |
| Selector               | `0x6e553f65` ✅                                                |
| 参数                     | `(uint256 _assets, address _receiver)`                        |
| `_assets`              | 底层资产数量（uint256）；传 `type(uint256).max` 时转入全部余额      |
| `_receiver`            | 接收 share 的地址                                               |
| 返回值                   | `uint256 shares_`                                             |
| 调用方式                  | onchainos wallet contract-call（需先 approve）                  |

---

#### 操作 11：发起提款请求 — requestRedeem（链上操作）

**用途**：发起异步赎回请求，将 share token 转给 rebalancer 托管，等待约 5 天后由 operator 执行真正的资产返还。这是 CIAN Vault 唯一支持的提款方式（标准 ERC4626 `withdraw`/`redeem` 已被禁用）。

| 字段                     | 值                                                                                    |
|------------------------|--------------------------------------------------------------------------------------|
| 合约                     | Vault Proxy（如 `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d`）                        |
| 函数签名（canonical）     | `requestRedeem(uint256,address)`                                                      |
| Selector               | `0x107703ab` ✅                                                                        |
| 参数                     | `(uint256 _shares, address _token)`                                                   |
| `_shares`              | 赎回的 share 数量（uint256）                                                             |
| `_token`               | 希望收到的资产 token 地址（如 stETH、WETH；支持范围视合约配置）                              |
| 返回值                   | 无（void）                                                                             |
| 调用方式                  | onchainos wallet contract-call（需用户签名）                                             |

> **关键约束**：
> - `withdraw()` 和 `redeem()` 会以 "Only delayed withdrawals are supported" 回滚。
> - 调用 `requestRedeem` 后，用户 share 被转移给 `redeemOperator`，用户无法单方面撤销。
> - 实际资产到账约 5 天后，由 rebalancer 链外触发 `optionalRedeem`（用户无需操作）。

**onchainos 命令示例**：
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d \
  --input-data <abi_encoded_calldata>
# calldata = 0x107703ab + abi.encode(shares_amount, stETH_address)
```

---

### 2.4 函数 Selector 汇总

| 函数                                            | Canonical 签名                                        | Selector     | 验证  |
|------------------------------------------------|------------------------------------------------------|--------------|------|
| ERC-20 Approve                                 | `approve(address,uint256)`                           | `0x095ea7b3` | ✅    |
| ERC-20 Allowance                               | `allowance(address,address)`                         | `0xdd62ed3e` | ✅    |
| Vault.asset                                    | `asset()`                                            | `0x38d52e0f` | ✅    |
| Vault.totalAssets                              | `totalAssets()`                                      | `0x01e1d114` | ✅    |
| Vault.totalSupply                              | `totalSupply()`                                      | `0x18160ddd` | ✅    |
| Vault.balanceOf                                | `balanceOf(address)`                                 | `0x70a08231` | ✅    |
| Vault.convertToAssets                          | `convertToAssets(uint256)`                           | `0x07a2d13a` | ✅    |
| Vault.convertToShares                          | `convertToShares(uint256)`                           | `0xc6e6f592` | ✅    |
| Vault.previewDeposit                           | `previewDeposit(uint256)`                            | `0xef8b30f7` | ✅    |
| Vault.previewRedeem                            | `previewRedeem(uint256)`                             | `0x4cdad506` | ✅    |
| Vault.exchangePrice                            | `exchangePrice()`                                    | `0x9e65741e` | ✅    |
| Vault.revenueExchangePrice                     | `revenueExchangePrice()`                             | `0x98e1862c` | ✅    |
| Vault.deposit (ERC4626)                        | `deposit(uint256,address)`                           | `0x6e553f65` | ✅    |
| Vault.mint (ERC4626)                           | `mint(uint256,address)`                              | `0x94bf804d` | ✅    |
| Vault.optionalDeposit                          | `optionalDeposit(address,uint256,address,address)`   | `0x32507a5f` | ✅    |
| Vault.requestRedeem                            | `requestRedeem(uint256,address)`                     | `0x107703ab` | ✅    |
| Vault.optionalRedeem (operator only)           | `optionalRedeem(address,uint256,uint256,address,address)` | `0xa7b73254` | ✅ |

---

## 3. 用户场景

### 场景 1：ETH 持有者存入 stETH Yield Layer 获取放大收益

**背景**：用户持有 ETH，希望通过 CIAN 的 Recursive Staking 策略在 stETH 上获取 8%-25% APY（包含 Lido 质押收益 + 杠杆放大 + 积分收益）。

**操作流程**：
1. **查询 Vault 信息**（REST API）：`GET /ethereum/pool/home` 获取 stETH Yield Layer APY 和 TVL 展示给用户
2. **查询当前持仓**（REST API 或 eth_call）：`GET /ethereum/pool/home/vault/user/{vault_addr}?user_address={user}` 查看现有 ylstETH 余额
3. **存入 ETH**（链上）：调用 `optionalDeposit(address(0), 0, receiver, address(0))`，附 `msg.value = amount_in_wei`
4. **确认**：查询 `balanceOf(user)` 或 REST API 用户持仓，确认 ylstETH 余额增加

**注意**：存入原生 ETH 无需 Approve 步骤。

---

### 场景 2：wstETH 持有者存入并查询实时资产价值

**背景**：用户持有 wstETH，希望存入 CIAN stETH Yield Layer，并实时查询自己的持仓价值。

**操作流程**：
1. **Approve**（链上）：调用 wstETH ERC-20 `approve(vault_proxy, amount)`
2. **存款**（链上）：调用 `optionalDeposit(wstETH_addr, amount, receiver, address(0))`，获取 ylstETH
3. **查询 share 余额**（eth_call）：`balanceOf(user)` → `shares`
4. **查询资产价值**（eth_call）：`convertToAssets(shares)` → 对应底层资产数量
5. **获取 exchangePrice**（eth_call）：`exchangePrice()` → 展示当前汇率趋势

---

### 场景 3：用户发起提款请求（异步赎回）

**背景**：用户持有 ylstETH，希望赎回为 stETH，了解并接受约 5 天等待期。

**操作流程**：
1. **查询持仓**（eth_call）：`balanceOf(user)` 确认持有 share 数量
2. **预览赎回**（eth_call）：`previewRedeem(shares)` 预估可获得的底层资产数量
3. **发起赎回**（链上）：调用 `requestRedeem(shares, stETH_addr)`
4. **告知用户**：赎回请求已提交，约 5 天后资产自动到账（无需再次操作）
5. **可选：跟踪状态**（REST API）：`GET /ethereum/pool/home/vault/user/{vault_addr}` 查看待处理赎回状态

> **用户教育要点**：
> - `requestRedeem` 后 share 立即转移给 rebalancer，用户不可撤销
> - rebalancer 约 5 天内执行 `optionalRedeem`，执行时可能扣除最高 1.2% 退出费
> - 如 rebalancer 长时间未执行（>5 天），建议联系 CIAN 支持

---

### 场景 4：查询所有 Vault 列表及最优收益选择

**背景**：用户希望了解 CIAN 在 Ethereum 上所有可用 Vault 的收益率，选择最优策略。

**操作流程**：
1. **获取 Vault 列表**（REST API）：`GET https://yieldlayer.cian.app/ethereum/pool/home`
2. **展示 APY 排行**：按 `apy_7`（7 日年化）降序展示各 Vault
3. **获取指定 Vault 详情**（REST API）：`GET /ethereum/pool/home/vault/breakdown/{pool_address}` 获取 APY 分解（基础 APY + Points + Eco Earn）
4. **告知用户底层资产**：调用目标 Vault 的 `asset()` 函数确认接受的底层资产类型

---

## 4. 外部 API 依赖

| 依赖类型               | Endpoint / 说明                                                              | 用途                          |
|----------------------|-----------------------------------------------------------------------------|-------------------------------|
| CIAN REST API        | `https://yieldlayer.cian.app/ethereum/pool/home`                            | 获取 Vault 列表、APY、TVL       |
| CIAN REST API        | `https://yieldlayer.cian.app/ethereum/pool/home/vault/user/{vault}?user_address={addr}` | 用户持仓查询              |
| CIAN REST API        | `https://yieldlayer.cian.app/ethereum/pool/home/vault/breakdown/{vault}`    | Vault APY 分解               |
| CIAN REST API        | `https://yieldlayer.cian.app/ethereum/pool/home/vault/history/`             | Vault 历史 TVL/APY 数据       |
| Ethereum RPC         | `https://ethereum.publicnode.com`（推荐）或 onchainos 内置                     | eth_call 只读查询、tx 广播     |
| 合约地址（硬编码）      | Vault Proxy 地址在插件 config.rs 中配置，来源于 Etherscan 验证合约               | 防止 DNS 劫持风险              |

> **RPC 注意**：Ethereum mainnet 请避免使用 `eth.llamarpc.com`（易 429），优先使用 `https://ethereum.publicnode.com`。

---

## 5. 配置参数

| 参数名                          | 类型     | 默认值                                           | 说明                                           |
|-------------------------------|--------|--------------------------------------------------|------------------------------------------------|
| `chain_id`                    | u64    | `1`                                              | Ethereum Mainnet chain ID                      |
| `rpc_url`                     | String | `https://ethereum.publicnode.com`                | Ethereum RPC 节点                              |
| `api_base_url`                | String | `https://yieldlayer.cian.app/ethereum`           | CIAN REST API 基础 URL                         |
| `vault_ylsteth`               | String | `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d`    | stETH Yield Layer Vault Proxy                  |
| `vault_ylpumpbtc`             | String | `0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b`    | pumpBTC Yield Layer Vault Proxy                |
| `default_referral`            | String | `address(0)`（全零地址）                           | 默认推荐人地址（无推荐时传零地址）                  |
| `max_exit_fee_bps`            | u64    | `120`                                            | 合约允许的最大退出费 1.2%（120/10000），仅供展示     |
| `withdrawal_wait_days`        | u64    | `5`                                              | 异步赎回预计等待天数，用于 UI 提示                 |

---

## 6. 实现注意事项

### 6.1 存款 Token 路由

`optionalDeposit` 对 ETH 和 ERC-20 的调用方式不同：

| 存入资产          | `_token` 参数          | `_assets` 参数      | `msg.value` |
|-----------------|----------------------|-------------------|-------------|
| 原生 ETH         | `address(0)`          | `0`               | 存款金额 wei   |
| WETH/stETH 等   | Token 合约地址          | 存款金额（原始精度）  | `0`         |

存 ERC-20 前务必先 Approve，否则 `transferFrom` 会 revert。建议先 `eth_call allowance(user, vault)` 检查，额度不足才发 approve 交易。

### 6.2 异步提款的用户预期管理

- 用户调用 `requestRedeem` 后 share **立即离开用户地址**，但资产约 5 天后才到账
- 插件须明确告知用户等待期，避免用户以为提款失败
- 不支持即时提款（`withdraw()`/`redeem()` 会 revert）

### 6.3 汇率精度

- `exchangePrice()` 初始值为 `1e18`，随时间增长（收益累积）
- 用户资产价值估算：`assets = shares * exchangePrice / 1e18`
- 展示前需按底层资产 decimals（WETH=18，pumpBTC 底层=8）格式化

### 6.4 Proxy 合约升级

两个 Vault 均为 `TransparentUpgradeableProxy`（EIP-1967）：
- 永远只调用 Proxy 地址，不调用 Implementation 地址
- 若合约行为异常（selector mismatch），优先检查实现合约是否已升级

### 6.5 价格更新限制

若链上 `exchangePrice` 超过 3 天未更新（由 rebalancer 触发 `updateExchangePrice`），Vault 会拒绝新的存款交易（revert）。此情况建议通过 REST API 确认 Vault 状态后再允许用户存款。

### 6.6 Approve 与 Deposit 间隔

参照 lending.md 的注意事项：同一秒内连续提交 approve 和 deposit 可能导致 nonce 冲突。建议在 approve tx 确认后（或至少 3 秒后）再提交 deposit 交易。

---

## 7. 参考资料

- [CIAN 官网](https://cian.app/)
- [CIAN Yield Layer 应用](https://yieldlayer.cian.app/)
- [CIAN 文档首页](https://docs.cian.app/)
- [Etherscan: ylstETH Vault Proxy](https://etherscan.io/token/0xB13aa2d0345b0439b064f26B82D8dCf3f508775d)
- [Etherscan: ylpumpBTC Vault Proxy](https://etherscan.io/token/0xd4cc9b31e9ef33e392ff2f81ad52be8523e0993b)
- [Etherscan: VaultYieldETH Implementation](https://etherscan.io/address/0xa1dc0b6a02ab091580dc57bdd5fe8a9e577e0842#code)
- [Ackee Blockchain — Cian Yield Layer Audit Summary](https://ackee.xyz/blog/cian-yield-layer-audit-summary/)
- [DefiLlama: CIAN Yield Layer](https://defillama.com/protocol/cian-yield-layer)
- [CIAN Medium: Yield Layer Overview](https://cian-app.medium.com/cians-yield-layer-overview-revolutionizing-defi-yield-generation-e5f49c39663e)
