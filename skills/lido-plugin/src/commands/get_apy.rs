use serde::Deserialize;

const DEFILLAMA_POOLS_URL: &str = "https://yields.llama.fi/pools";

#[derive(Debug, Deserialize)]
struct PoolsResponse {
    data: Vec<PoolItem>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct PoolItem {
    chain: Option<String>,
    project: Option<String>,
    symbol: Option<String>,
    tvlUsd: Option<f64>,
    apy: Option<f64>,
    apyPct1D: Option<f64>,
    apyPct7D: Option<f64>,
    apyPct30D: Option<f64>,
    apyMean30d: Option<f64>,
}

fn is_target_pool(pool: &PoolItem) -> bool {
    let project = pool.project.as_deref().unwrap_or("").to_ascii_lowercase();
    let symbol = pool.symbol.as_deref().unwrap_or("").to_ascii_lowercase();
    let chain = pool.chain.as_deref().unwrap_or("").to_ascii_lowercase();

    project == "lido"
        && chain == "ethereum"
        && (symbol.contains("steth") || symbol.contains("wsteth"))
}

fn pick_best_pool(mut pools: Vec<PoolItem>) -> Option<PoolItem> {
    pools.sort_by(|a, b| {
        let atvl = a.tvlUsd.unwrap_or(0.0);
        let btvl = b.tvlUsd.unwrap_or(0.0);
        btvl.partial_cmp(&atvl).unwrap_or(std::cmp::Ordering::Equal)
    });
    pools.into_iter().next()
}

fn fmt_pct(v: Option<f64>) -> String {
    v.map(|x| format!("{:.3}%", x)).unwrap_or_else(|| "N/A".to_string())
}

fn fmt_usd(v: Option<f64>) -> String {
    match v {
        Some(x) if x >= 1_000_000_000.0 => format!("${:.2}B", x / 1_000_000_000.0),
        Some(x) if x >= 1_000_000.0 => format!("${:.2}M", x / 1_000_000.0),
        Some(x) => format!("${:.0}", x),
        None => "N/A".to_string(),
    }
}

pub async fn run() -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let resp: PoolsResponse = client
        .get(DEFILLAMA_POOLS_URL)
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let matches: Vec<PoolItem> = resp.data.into_iter().filter(is_target_pool).collect();

    let pool = pick_best_pool(matches)
        .ok_or_else(|| anyhow::anyhow!("No Lido stETH pool found on DeFiLlama"))?;

    println!("=== Lido stETH APY (via DeFiLlama) ===");
    println!("Asset:       {}", pool.symbol.as_deref().unwrap_or("N/A"));
    println!("APY:         {}", fmt_pct(pool.apy));
    println!("TVL:         {}", fmt_usd(pool.tvlUsd));
    println!("1D change:   {}", fmt_pct(pool.apyPct1D));
    println!("7D change:   {}", fmt_pct(pool.apyPct7D));
    println!("30D change:  {}", fmt_pct(pool.apyPct30D));
    println!("30D avg APY: {}", fmt_pct(pool.apyMean30d));
    println!();
    println!("Note: Data sourced from DeFiLlama (third-party aggregator).");
    println!("      This is post-10%-fee rate. Rewards compound daily.");

    Ok(())
}
