/// `fourmeme-plugin list-tokens` — discover tokens via four.meme public APIs.
///
/// Two modes:
///   - default (no --keyword): POST /token/ranking with --type
///   - with --keyword: POST /token/search with --keyword + --type
///
/// `--type` accepts native ranking types (HOT, NEW, CAP, PROGRESS, VOL,
/// VOL_DAY_1, VOL_HOUR_4, VOL_HOUR_1, VOL_MIN_30, VOL_MIN_5, LAST, DEX, BURN).

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct ListTokensArgs {
    /// Ranking type (default HOT). One of HOT, NEW, CAP, PROGRESS, VOL, VOL_DAY_1,
    /// VOL_HOUR_4, VOL_HOUR_1, VOL_MIN_30, VOL_MIN_5, LAST, DEX, BURN.
    #[arg(long, default_value = "HOT")]
    pub r#type: String,

    /// Optional keyword — switches to search mode if set.
    #[arg(long)]
    pub keyword: Option<String>,

    /// Result count (1..=100, default 20)
    #[arg(long, default_value_t = 20)]
    pub limit: u32,

    /// Search mode: page index (default 1; ignored without --keyword)
    #[arg(long, default_value_t = 1)]
    pub page: u32,
}

pub async fn run(args: ListTokensArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("list-tokens"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: ListTokensArgs) -> Result<()> {
    let limit = args.limit.clamp(1, 100);

    let (mode, items) = match args.keyword.as_deref() {
        Some(kw) if !kw.is_empty() => {
            ("search",
             crate::api::fetch_token_search(kw, &args.r#type, args.page, limit).await?)
        }
        _ => {
            ("ranking",
             crate::api::fetch_token_ranking(&args.r#type, limit).await?)
        }
    };

    let rows: Vec<serde_json::Value> = items.iter().map(|t| {
        // Both endpoints return the same token shape
        serde_json::json!({
            "token":        t["tokenAddress"],
            "name":         t["name"],
            "symbol":       t["shortName"],
            "quote":        t["symbol"],            // BNB / USDT / etc.
            "price":        t["price"],
            "market_cap":   t["cap"].as_str().or_else(|| t["marketCap"].as_str()).unwrap_or("0"),
            "progress":     t["progress"],
            "volume_24h":   t.get("day1Vol").or_else(|| t.get("volume")),
            "increase_24h": t["day1Increase"].as_str()
                              .or_else(|| t["increase"].as_str()).unwrap_or("0"),
            "img":          t["img"],
            "version":      t["version"],
            "status":       t["status"],
            "ai_creator":   t.get("aiCreator").unwrap_or(&serde_json::Value::Bool(false)),
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "mode": mode,
            "type": args.r#type,
            "keyword": args.keyword,
            "count": rows.len(),
            "tokens": rows,
            "tip": "Use the `token` field of any row with `get-token --address <token>` or `quote-buy --token <token> --funds 0.005` to drill in.",
        }
    }))?);
    Ok(())
}
