use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct VaultsArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Owner address (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: VaultsArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    let owner = match args.address {
        Some(a) => a,
        None => onchainos::resolve_wallet(chain_id).unwrap_or_else(|_| String::new()),
    };

    if owner.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --address or ensure onchainos is logged in.");
    }

    println!("=== Sky Lending — CDP Vaults ===");
    println!("Owner:  {}", owner);
    println!("Chain:  {}", chain_id);
    println!();

    // Get count of CDPs for this owner
    let count_calldata = rpc::calldata_single_address(config::SEL_CDP_COUNT, &owner);
    let count_result = onchainos::eth_call(chain_id, config::CDP_MANAGER, &count_calldata)?;
    let count = match rpc::extract_return_data(&count_result) {
        Ok(hex) => rpc::decode_uint256(&hex).unwrap_or(0),
        Err(_) => 0,
    };

    println!("Total CDPs: {}", count);
    println!();

    if count == 0 {
        println!("No CDP vaults found for this address.");
        println!("Use 'sky-lending open-vault' to create a new vault.");
        return Ok(());
    }

    // Walk the doubly-linked list starting from first(owner)
    let first_calldata = rpc::calldata_single_address(config::SEL_CDP_FIRST, &owner);
    let first_result = onchainos::eth_call(chain_id, config::CDP_MANAGER, &first_calldata)?;
    let mut cdp_id = match rpc::extract_return_data(&first_result) {
        Ok(hex) => rpc::decode_uint256(&hex).unwrap_or(0),
        Err(_) => 0,
    };

    let mut vault_num = 1;
    while cdp_id != 0 {
        println!("--- Vault #{} (CDP ID: {}) ---", vault_num, cdp_id);

        // Get urn address for this cdpId
        let urn_calldata = rpc::calldata_single_uint256(config::SEL_CDP_URNS, cdp_id as u64);
        let urn_result = onchainos::eth_call(chain_id, config::CDP_MANAGER, &urn_calldata)?;
        let urn = match rpc::extract_return_data(&urn_result) {
            Ok(hex) => rpc::decode_address(&hex).unwrap_or_else(|_| "unknown".to_string()),
            Err(_) => "unknown".to_string(),
        };

        // Get ilk for this cdpId
        let ilk_calldata = rpc::calldata_single_uint256(config::SEL_CDP_ILKS, cdp_id as u64);
        let ilk_result = onchainos::eth_call(chain_id, config::CDP_MANAGER, &ilk_calldata)?;
        let ilk_bytes32 = match rpc::extract_return_data(&ilk_result) {
            Ok(hex) => rpc::decode_bytes32(&hex).unwrap_or_else(|_| String::new()),
            Err(_) => String::new(),
        };
        let ilk_name = rpc::bytes32_to_str(&ilk_bytes32);

        println!("  Ilk:      {}", if ilk_name.is_empty() { "unknown".to_string() } else { ilk_name.clone() });
        println!("  Urn:      {}", urn);

        // Get vault state from Vat.urns(ilk, urn)
        if !ilk_bytes32.is_empty() && urn != "unknown" && urn != "0x0000000000000000000000000000000000000000" {
            let vat_urns_calldata = rpc::calldata_vat_urns(&ilk_bytes32, &urn);
            match onchainos::eth_call(chain_id, config::VAT, &vat_urns_calldata) {
                Ok(vat_result) => {
                    match rpc::extract_return_data(&vat_result) {
                        Ok(hex) => {
                            match rpc::decode_two_uint256(&hex) {
                                Ok((ink, art)) => {
                                    let ink_f = ink as f64 / 1e18;

                                    // Get rate for this ilk to compute actual DAI debt (rate in RAY = 1e27)
                                    let vat_ilks_calldata = rpc::calldata_vat_ilks(&ilk_bytes32);
                                    let rate_f64 = match onchainos::eth_call(chain_id, config::VAT, &vat_ilks_calldata) {
                                        Ok(r) => {
                                            match rpc::extract_return_data(&r) {
                                                Ok(hex2) => rpc::decode_five_uint256_f64(&hex2).map(|(_, rate, _, _, _)| rate).unwrap_or(1e27),
                                                Err(_) => 1e27,
                                            }
                                        }
                                        Err(_) => 1e27,
                                    };

                                    // art * rate / 1e27 / 1e18 = DAI in display units
                                    let dai_debt_f = (art as f64) * rate_f64 / 1e27 / 1e18;

                                    println!("  Collateral: {:.6} {} (ink: {})", ink_f, ilk_name, ink);
                                    println!("  DAI Debt:   {:.6} DAI (art: {})", dai_debt_f, art);

                                    if ink > 0 && dai_debt_f > 0.0 {
                                        println!("  Note: Check collateralization ratio to avoid liquidation.");
                                    } else if art == 0 {
                                        println!("  Status:     No debt (empty vault)");
                                    }
                                }
                                Err(e) => println!("  (decode error: {})", e),
                            }
                        }
                        Err(e) => println!("  (error fetching vault state: {})", e),
                    }
                }
                Err(e) => println!("  (RPC error fetching vault state: {})", e),
            }
        }

        println!();

        // Walk linked list: list(cdpId) -> (prev, next)
        let list_calldata = rpc::calldata_single_uint256(config::SEL_CDP_LIST, cdp_id as u64);
        match onchainos::eth_call(chain_id, config::CDP_MANAGER, &list_calldata) {
            Ok(list_result) => {
                match rpc::extract_return_data(&list_result) {
                    Ok(hex) => {
                        match rpc::decode_two_uint256(&hex) {
                            Ok((_prev, next)) => {
                                cdp_id = next;
                            }
                            Err(_) => break,
                        }
                    }
                    Err(_) => break,
                }
            }
            Err(_) => break,
        }

        vault_num += 1;
        if vault_num > 50 {
            println!("(Stopped at 50 vaults)");
            break;
        }
    }

    Ok(())
}
