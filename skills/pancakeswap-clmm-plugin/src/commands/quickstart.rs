const MIN_GAS_WEI: u128 = 5_000_000_000_000_000; // 0.005 BNB

pub async fn run() -> anyhow::Result<()> {
    eprintln!("Checking wallet on BSC...");

    match crate::onchainos::resolve_wallet(56).await {
        Ok(wallet) if !wallet.is_empty() => {
            let gas_balance = crate::onchainos::get_native_balance(56, &wallet).await.unwrap_or(0);
            let has_gas = gas_balance >= MIN_GAS_WEI;

            let status = if has_gas { "ready" } else { "needs_gas" };
            let suggestion = if has_gas {
                "Stake a V3 LP NFT to earn CAKE rewards, or view your existing positions.".to_string()
            } else {
                format!(
                    "Your wallet has insufficient BNB for gas. Send at least 0.005 BNB to {}.",
                    wallet
                )
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "about": "PancakeSwap V3 CLMM plugin — stake LP NFTs, harvest CAKE rewards, and collect swap fees across BSC, Ethereum, Base, and Arbitrum.",
                    "wallet": wallet,
                    "chain": "BSC",
                    "chain_id": 56,
                    "status": status,
                    "suggestion": suggestion,
                    "next_command": "pancakeswap-clmm-plugin positions",
                    "onboarding_steps": [
                        "1. View your V3 LP positions:",
                        "   pancakeswap-clmm-plugin positions",
                        "2. Check active farming pools:",
                        "   pancakeswap-clmm-plugin farm-pools",
                        "3. Preview staking a position (replace 12345 with your token ID, no transaction):",
                        "   pancakeswap-clmm-plugin --chain 56 farm --token-id 12345",
                        "   (Add --confirm to execute)",
                        "4. Check pending CAKE rewards:",
                        "   pancakeswap-clmm-plugin pending-rewards --token-id 12345",
                        "5. Preview harvesting CAKE rewards (no transaction):",
                        "   pancakeswap-clmm-plugin --chain 56 harvest --token-id 12345",
                        "   (Add --confirm to execute)"
                    ]
                }))?
            );
        }
        _ => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": "No wallet found. Run: onchainos wallet login your@email.com"
                }))?
            );
        }
    }

    Ok(())
}
