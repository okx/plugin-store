/// Pendle RouterV4 address — same across all supported chains
pub const PENDLE_ROUTER: &str = "0x888888888889758F76e7103c6CbF23ABbF58F946";

/// Pendle API base URL
pub const PENDLE_API_BASE: &str = "https://api-v2.pendle.finance/core";

/// Return RPC URL for the given chain ID
pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "https://cloudflare-eth.com",
        56 => "https://bsc-rpc.publicnode.com",
        8453 => "https://base-rpc.publicnode.com",
        42161 => "https://arb1.arbitrum.io/rpc",
        _ => "https://cloudflare-eth.com",
    }
}
