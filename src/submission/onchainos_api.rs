/// onchainos CLI capability registry.
///
/// This module defines what onchainos provides so that lint and AI review can
/// detect plugins that re-implement capabilities instead of using the CLI.
///
/// Source of truth: onchainos-skills-main/cli/src/ (audit.rs command map)

/// Every onchainos subcommand, grouped by top-level command.
pub const ONCHAINOS_COMMANDS: &[(&str, &[&str])] = &[
    (
        "token",
        &[
            "search",
            "info",
            "holders",
            "trending",
            "price-info",
            "liquidity",
            "hot-tokens",
            "advanced-info",
            "top-trader",
            "trades",
            "cluster-overview",
            "cluster-top-holders",
            "cluster-list",
            "cluster-supported-chains",
        ],
    ),
    (
        "market",
        &[
            "price",
            "prices",
            "kline",
            "index",
            "portfolio-supported-chains",
            "portfolio-overview",
            "portfolio-dex-history",
            "portfolio-recent-pnl",
            "portfolio-token-pnl",
            "address-tracker-activities",
        ],
    ),
    (
        "swap",
        &["quote", "swap", "approve", "chains", "liquidity"],
    ),
    (
        "gateway",
        &["gas", "gas-limit", "simulate", "broadcast", "orders", "chains"],
    ),
    (
        "portfolio",
        &["chains", "total-value", "all-balances", "token-balances"],
    ),
    (
        "wallet",
        &[
            "login",
            "verify",
            "add",
            "switch",
            "status",
            "addresses",
            "logout",
            "chains",
            "balance",
            "send",
            "history",
            "contract-call",
        ],
    ),
    (
        "security",
        &["token-scan", "dapp-scan", "tx-scan", "approvals", "sig-scan"],
    ),
    (
        "signal",
        &["chains", "list"],
    ),
    (
        "memepump",
        &[
            "chains",
            "tokens",
            "token-details",
            "token-dev-info",
            "similar-tokens",
            "token-bundle-info",
            "aped-wallet",
        ],
    ),
    (
        "leaderboard",
        &["supported-chains", "list"],
    ),
    (
        "payment",
        &["x402-pay"],
    ),
];

/// Capability categories that onchainos covers.
/// Each category maps to patterns that indicate a plugin is self-implementing
/// instead of using onchainos.
pub struct BypassPattern {
    /// What capability is being self-implemented
    pub capability: &'static str,
    /// The onchainos command(s) that should be used instead
    pub onchainos_alternative: &'static str,
    /// Patterns to detect in SKILL.md (case-insensitive)
    pub patterns: &'static [&'static str],
    /// Severity: "error" = must use onchainos, "warning" = strongly recommended
    pub severity: &'static str,
}

/// Patterns that indicate a plugin is bypassing onchainos and doing things itself.
pub const BYPASS_PATTERNS: &[BypassPattern] = &[
    // ── Price / Market Data ──────────────────────────────────────
    BypassPattern {
        capability: "Price query",
        onchainos_alternative: "onchainos token price-info / market price",
        patterns: &[
            "coingecko.com/api",
            "api.coingecko.com",
            "dexscreener.com/api",
            "api.dexscreener.com",
            "birdeye.so/api",
            "api.birdeye.so",
            "api.geckoterminal.com",
            "defined.fi/api",
            "api.defined.fi",
            "api.coinmarketcap.com",
            "pro-api.coinmarketcap.com",
        ],
        severity: "warning",
    },
    // ── DEX / Swap ───────────────────────────────────────────────
    BypassPattern {
        capability: "DEX swap / quote",
        onchainos_alternative: "onchainos swap quote / swap swap",
        patterns: &[
            "jup.ag/api",
            "quote-api.jup.ag",
            "api.jupiter.ag",
            "api.1inch.dev",
            "api.0x.org",
            "api.paraswap.io",
            "router.uniswap",
            "api.odos.xyz",
        ],
        severity: "warning",
    },
    // ── Blockchain RPC ───────────────────────────────────────────
    BypassPattern {
        capability: "Direct blockchain RPC",
        onchainos_alternative: "onchainos gateway / token / market commands",
        patterns: &[
            "api.mainnet-beta.solana.com",
            "mainnet.helius-rpc.com",
            "rpc.ankr.com",
            "eth-mainnet.g.alchemy.com",
            "mainnet.infura.io",
            "rpc.quicknode.com",
            "getblock.io",
            "jsonrpc",
            "eth_call",
            "eth_sendTransaction",
            "eth_getBalance",
            "getAccountInfo",
            "getTokenAccountsByOwner",
            "simulateTransaction",
        ],
        severity: "error",
    },
    // ── Wallet / Signing ─────────────────────────────────────────
    BypassPattern {
        capability: "Wallet management / signing",
        onchainos_alternative: "onchainos wallet (login, send, sign, contract-call)",
        patterns: &[
            "private_key",
            "private key",
            "privatekey",
            "secret_key",
            "secret key",
            "mnemonic",
            "seed phrase",
            "seed_phrase",
            "keystore",
            "wallet.json",
            "signTransaction",
            "sign_transaction",
            "signMessage",
            "sign_message",
            "eth_sign",
            "personal_sign",
            "signTypedData",
        ],
        severity: "error",
    },
    // ── Web3 Libraries ───────────────────────────────────────────
    BypassPattern {
        capability: "Web3 library usage",
        onchainos_alternative: "onchainos CLI commands",
        patterns: &[
            "ethers.js",
            "ethers.providers",
            "new ethers.",
            "web3.js",
            "new Web3(",
            "web3.eth.",
            "@solana/web3.js",
            "solana/web3",
            "Connection(",
            "from viem",
            "import viem",
            "from web3",
            "import web3",
            "alloy::",
            "ethers::",
        ],
        severity: "warning",
    },
    // ── Transaction Building ─────────────────────────────────────
    BypassPattern {
        capability: "Transaction construction",
        onchainos_alternative: "onchainos gateway simulate / broadcast",
        patterns: &[
            "buildTransaction",
            "build_transaction",
            "serializeTransaction",
            "serialize_transaction",
            "rlp.encode",
            "Transaction.from",
            "Transaction({",
            "SystemProgram.transfer",
            "SystemInstruction",
        ],
        severity: "error",
    },
    // ── Token Approvals / Contract Calls ─────────────────────────
    BypassPattern {
        capability: "Direct contract interaction",
        onchainos_alternative: "onchainos swap approve / wallet contract-call",
        patterns: &[
            "contract.methods",
            "contract.functions",
            "abi.encode",
            "encodeFunctionData",
            "encode_abi",
            "approve(address",
            "transferFrom(",
            "safeTransferFrom",
        ],
        severity: "error",
    },
    // ── Security Scanning ────────────────────────────────────────
    BypassPattern {
        capability: "Security scanning",
        onchainos_alternative: "onchainos security (token-scan, dapp-scan, tx-scan)",
        patterns: &[
            "gopluslabs.io",
            "api.gopluslabs.io",
            "honeypot.is",
            "tokensniffer.com",
            "rugdoc.io",
        ],
        severity: "warning",
    },
];

/// Check if a given `onchainos <top> <sub>` command exists.
pub fn command_exists(top: &str, sub: &str) -> bool {
    ONCHAINOS_COMMANDS
        .iter()
        .any(|(t, subs)| *t == top && subs.contains(&sub))
}

/// Check if a top-level command exists.
pub fn top_command_exists(top: &str) -> bool {
    ONCHAINOS_COMMANDS.iter().any(|(t, _)| *t == top)
}

/// Return all top-level commands.
pub fn top_commands() -> Vec<&'static str> {
    ONCHAINOS_COMMANDS.iter().map(|(t, _)| *t).collect()
}

/// Return a flat list of all "top sub" command strings.
pub fn all_commands_flat() -> Vec<String> {
    ONCHAINOS_COMMANDS
        .iter()
        .flat_map(|(top, subs)| subs.iter().map(move |sub| format!("{} {}", top, sub)))
        .collect()
}
