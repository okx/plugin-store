use crate::abi::{selector, calldata, encode_address, word, word_to_u128};
use crate::chain::eth_call;
use crate::contracts::NFT_CONTRACT;

/// Get the number of NFT positions owned by a wallet.
pub async fn balance_of(chain_id: u64, owner: &str) -> anyhow::Result<u64> {
    let sel = selector("balanceOf(address)");
    let data = calldata(sel, &[encode_address(owner)]);
    let result = eth_call(chain_id, NFT_CONTRACT, &data).await?;
    let w = word(&result, 0).ok_or_else(|| anyhow::anyhow!("No result for balanceOf"))?;
    Ok(word_to_u128(&w) as u64)
}

/// Get the NFT token ID at a specific index for an owner.
pub async fn token_of_owner_by_index(chain_id: u64, owner: &str, index: u64) -> anyhow::Result<u64> {
    let sel = selector("tokenOfOwnerByIndex(address,uint256)");
    let mut idx_word = [0u8; 32];
    idx_word[24..].copy_from_slice(&index.to_be_bytes());
    let data = calldata(sel, &[encode_address(owner), idx_word]);
    let result = eth_call(chain_id, NFT_CONTRACT, &data).await?;
    let w = word(&result, 0).ok_or_else(|| anyhow::anyhow!("No result for tokenOfOwnerByIndex"))?;
    Ok(word_to_u128(&w) as u64)
}

/// Get all NFT IDs owned by a wallet (up to max_count).
pub async fn nft_ids_of(chain_id: u64, owner: &str, max_count: u64) -> anyhow::Result<Vec<u64>> {
    let balance = balance_of(chain_id, owner).await?;
    let count = balance.min(max_count);
    let mut ids = Vec::with_capacity(count as usize);
    for i in 0..count {
        match token_of_owner_by_index(chain_id, owner, i).await {
            Ok(id) => ids.push(id),
            Err(_) => break,
        }
    }
    Ok(ids)
}
