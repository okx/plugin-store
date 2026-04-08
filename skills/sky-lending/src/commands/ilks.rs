use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct IlksArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,
}

pub async fn run(args: IlksArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    println!("=== Sky Lending — Collateral Types (Ilks) ===");
    println!("Chain: {}", chain_id);
    println!();

    for (name, ilk_hex) in config::KNOWN_ILKS {
        println!("--- {} ---", name);

        // Vat.ilks(bytes32) -> (Art, rate, spot, line, dust)
        let calldata = rpc::calldata_vat_ilks(ilk_hex);
        match onchainos::eth_call(chain_id, config::VAT, &calldata) {
            Ok(result) => {
                match rpc::extract_return_data(&result) {
                    Ok(hex) => {
                        match rpc::decode_five_uint256_f64(&hex) {
                            Ok((art, rate, spot, line, dust)) => {
                                // art is normalized debt (wad), rate is in RAY (1e27)
                                // total DAI = art * rate / 1e27
                                let total_dai = art * rate / 1e27;
                                // spot is liquidation price per collateral unit in RAY
                                let spot_f = spot / 1e27;
                                // line is max debt ceiling in RAD (1e45)
                                let line_dai = line / 1e45;
                                // dust is min vault debt in RAD
                                let dust_dai = dust / 1e45;
                                // rate in ray: stability fee accumulator (~1.0 = no fees accrued)
                                let rate_f = rate / 1e27;

                                println!("  Total Debt:       {:.2} DAI", total_dai / 1e18);
                                println!("  Rate (accum):     {:.8}", rate_f);
                                println!("  Spot Price:       {:.4} DAI/collateral (liq price)", spot_f);
                                println!("  Debt Ceiling:     {:.2} DAI", line_dai);
                                println!("  Min Vault Debt:   {:.2} DAI", dust_dai);
                            }
                            Err(e) => println!("  (decode error: {})", e),
                        }
                    }
                    Err(e) => println!("  (error fetching Vat.ilks: {})", e),
                }
            }
            Err(e) => println!("  (RPC error: {})", e),
        }

        // Jug.ilks(bytes32) -> (duty, rho)
        let jug_calldata = rpc::calldata_jug_ilks(ilk_hex);
        match onchainos::eth_call(chain_id, config::JUG, &jug_calldata) {
            Ok(result) => {
                match rpc::extract_return_data(&result) {
                    Ok(hex) => {
                        if let Ok((duty_f64, _rho, _, _, _)) = rpc::decode_five_uint256_f64(&hex) {
                            // duty in ray (1e27); annualized: (duty/1e27)^seconds_per_year - 1
                            let duty_normalized = duty_f64 / 1e27;
                            // Approximate: (duty_normalized - 1.0) * seconds_per_year * 100
                            let per_second_excess = duty_normalized - 1.0;
                            let annual_rate = per_second_excess * 31_536_000.0 * 100.0;
                            println!("  Stability Fee:    {:.4}% per year (approx linear)", annual_rate);
                        }
                    }
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }

        println!();
    }

    Ok(())
}
