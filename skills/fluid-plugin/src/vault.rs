use crate::abi::{selector, calldata, calldata_raw, encode_address_array, encode_address, word, word_to_address, word_to_bool, word_to_i128};
use crate::chain::{eth_call, eth_get_logs, eth_get_transaction};
use crate::contracts::*;

#[derive(Debug, Clone)]
pub struct VaultInfo {
    pub address: String,
    pub is_smart_col: bool,
    pub is_smart_debt: bool,
    pub col_token: String,
    pub debt_token: String,
    /// Vault borrow rate in basis-point-like units: 1% = 100. Divide by 100.0 to display as %.
    pub borrow_rate_vault: i64,
}

/// Fetch all vault addresses from the resolver.
pub async fn all_vault_addresses(chain_id: u64) -> anyhow::Result<Vec<String>> {
    let sel = selector("getAllVaultsAddresses()");
    let data = calldata(sel, &[]);
    let result = eth_call(chain_id, VAULT_RESOLVER, &data).await?;

    // ABI: (address[]) = offset(32) + length + elements
    if result.len() < 128 {
        return Ok(vec![]);
    }
    let length = usize::from_str_radix(&result[64..128], 16).unwrap_or(0);
    let mut addresses = Vec::with_capacity(length);
    for i in 0..length {
        let start = 128 + i * 64;
        if result.len() < start + 64 {
            break;
        }
        let w_bytes: [u8; 32] = hex::decode(&result[start..start + 64])
            .unwrap_or_default()
            .try_into()
            .unwrap_or([0u8; 32]);
        addresses.push(word_to_address(&w_bytes));
    }
    Ok(addresses)
}

/// Fetch vault info for multiple vaults in a single resolver call.
/// Uses getVaultsEntireData(address[]) which returns packed fixed-size structs.
pub async fn vault_infos_batch(chain_id: u64, addresses: &[String]) -> anyhow::Result<Vec<VaultInfo>> {
    if addresses.is_empty() {
        return Ok(vec![]);
    }
    let sel = selector("getVaultsEntireData(address[])");
    let addr_refs: Vec<&str> = addresses.iter().map(|s| s.as_str()).collect();
    let payload = encode_address_array(&addr_refs);
    let data = calldata_raw(sel, &payload);

    let result = eth_call(chain_id, VAULT_RESOLVER, &data).await?;

    // ABI: offset(32) + length + N × VAULT_STRUCT_WORDS words
    if result.len() < 128 {
        return Ok(vec![]);
    }
    let length = usize::from_str_radix(&result[64..128], 16).unwrap_or(0);
    let mut infos = Vec::with_capacity(length);

    for i in 0..length {
        let base = 128 + i * VAULT_STRUCT_WORDS * 64;
        if result.len() < base + VAULT_STRUCT_WORDS * 64 {
            break;
        }
        let get = |slot: usize| -> Option<[u8; 32]> {
            let start = base + slot * 64;
            let mut w = [0u8; 32];
            hex::decode_to_slice(&result[start..start + 64], &mut w).ok()?;
            Some(w)
        };

        let is_smart_col = get(WORD_IS_SMART_COL).map(|w| word_to_bool(&w)).unwrap_or(false);
        let is_smart_debt = get(WORD_IS_SMART_DEBT).map(|w| word_to_bool(&w)).unwrap_or(false);
        let col_token = get(WORD_COL_TOKEN).map(|w| word_to_address(&w)).unwrap_or_default();
        let debt_token = get(WORD_DEBT_TOKEN).map(|w| word_to_address(&w)).unwrap_or_default();
        let borrow_rate_vault = get(WORD_BORROW_RATE_VAULT)
            .map(|w| word_to_i128(&w) as i64)
            .unwrap_or(0);

        infos.push(VaultInfo {
            address: addresses[i].clone(),
            is_smart_col,
            is_smart_debt,
            col_token,
            debt_token,
            borrow_rate_vault,
        });
    }
    Ok(infos)
}

/// Fetch single vault info.
pub async fn vault_info_single(chain_id: u64, vault: &str) -> anyhow::Result<VaultInfo> {
    let sel = selector("getVaultEntireData(address)");
    let data = calldata(sel, &[encode_address(vault)]);
    let result = eth_call(chain_id, VAULT_RESOLVER, &data).await?;

    if result.len() < VAULT_STRUCT_WORDS * 64 {
        anyhow::bail!("Unexpected vault data length for {}", vault);
    }
    let get = |slot: usize| -> Option<[u8; 32]> { word(&result, slot) };

    Ok(VaultInfo {
        address: vault.to_lowercase(),
        is_smart_col: get(WORD_IS_SMART_COL).map(|w| word_to_bool(&w)).unwrap_or(false),
        is_smart_debt: get(WORD_IS_SMART_DEBT).map(|w| word_to_bool(&w)).unwrap_or(false),
        col_token: get(WORD_COL_TOKEN).map(|w| word_to_address(&w)).unwrap_or_default(),
        debt_token: get(WORD_DEBT_TOKEN).map(|w| word_to_address(&w)).unwrap_or_default(),
        borrow_rate_vault: get(WORD_BORROW_RATE_VAULT)
            .map(|w| word_to_i128(&w) as i64)
            .unwrap_or(0),
    })
}

/// Vault type label.
pub fn vault_type(info: &VaultInfo) -> &'static str {
    match (info.is_smart_col, info.is_smart_debt) {
        (false, false) => "T1",
        (true, false)  => "T2",
        (false, true)  => "T2",
        (true, true)   => "T3",
    }
}

/// Get the vault address for a given NFT ID via VaultResolver (works on ETH, fails on ARB).
async fn vault_for_nft_resolver(chain_id: u64, nft_id: u64) -> anyhow::Result<String> {
    let sel = selector("getVaultAddressFromNftId(uint256)");
    let data = calldata(sel, &[{
        let mut w = [0u8; 32];
        w[24..].copy_from_slice(&nft_id.to_be_bytes());
        w
    }]);
    let result = eth_call(chain_id, VAULT_RESOLVER, &data).await?;
    let w = word(&result, 0).ok_or_else(|| anyhow::anyhow!("No result for vault_for_nft"))?;
    let addr = word_to_address(&w);
    if addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!("VaultResolver returned zero address for NFT #{}", nft_id);
    }
    Ok(addr)
}

/// Get the vault address for a given NFT by finding the mint transaction.
/// When a user calls operate(0, ...) on a vault, the vault mints the NFT.
/// The transaction's `to` field is the vault address.
async fn vault_for_nft_from_mint_tx(chain_id: u64, nft_id: u64, owner: &str) -> anyhow::Result<String> {
    // ERC-721 Transfer(address indexed from, address indexed to, uint256 indexed tokenId)
    // topic0 = keccak256("Transfer(address,address,uint256)")
    let transfer_topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
    // topic1 = from = 0x0 (mint)
    let from_topic = "0x0000000000000000000000000000000000000000000000000000000000000000";
    // topic2 = to = owner (padded)
    let owner_hex = owner.trim_start_matches("0x");
    let to_topic = format!("0x{:0>64}", owner_hex.to_lowercase());
    // topic3 = tokenId = nft_id (padded)
    let token_topic = format!("0x{:064x}", nft_id);

    let logs = eth_get_logs(
        chain_id,
        NFT_CONTRACT,
        &[Some(transfer_topic), Some(from_topic), Some(to_topic.as_str()), Some(token_topic.as_str())],
    ).await?;

    if logs.is_empty() {
        anyhow::bail!(
            "No mint event found for NFT #{} on chain {}. \
             The position may belong to a different owner or chain.",
            nft_id, chain_id
        );
    }

    let tx_hash = &logs[0].transaction_hash;
    let tx = eth_get_transaction(chain_id, tx_hash).await?;
    tx.to.ok_or_else(|| anyhow::anyhow!("Mint tx {} has no `to` field", tx_hash))
}

/// Get the vault address for a given NFT ID.
/// Tries the VaultResolver first (fast, works on ETH). Falls back to mint-tx lookup (works on ARB).
pub async fn vault_for_nft(chain_id: u64, nft_id: u64, owner: &str) -> anyhow::Result<String> {
    match vault_for_nft_resolver(chain_id, nft_id).await {
        Ok(addr) => return Ok(addr),
        Err(_) => {}
    }
    vault_for_nft_from_mint_tx(chain_id, nft_id, owner).await
}
