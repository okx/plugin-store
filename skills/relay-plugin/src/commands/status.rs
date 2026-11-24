use clap::Args;
use crate::api::get_status;

#[derive(Args)]
pub struct StatusArgs {
    /// Request ID from a previous bridge command
    #[arg(long)]
    pub request_id: String,
}

pub async fn run(args: StatusArgs) -> anyhow::Result<()> {
    let status = get_status(&args.request_id).await?;

    let origin_tx = status.in_tx_hashes.as_ref()
        .and_then(|txs| txs.first())
        .map(|s| s.as_str())
        .unwrap_or("pending");
    let dest_tx = status.tx_hashes.as_ref()
        .and_then(|txs| txs.first())
        .map(|s| s.as_str())
        .unwrap_or("pending");

    let out = serde_json::json!({
        "status":      status.status,
        "request_id":  args.request_id,
        "origin_tx":   origin_tx,
        "dest_tx":     dest_tx,
        "error":       status.error.as_deref().unwrap_or(""),
    });

    println!("{}", serde_json::to_string_pretty(&out)?);

    // "unknown" means not yet indexed — this is a normal transient state,
    // not an error. Print a hint on stderr but exit 0 so callers can pipe the JSON.
    if status.status == "unknown" {
        eprintln!("[relay] Request not yet indexed. Wait a few seconds and try again.");
    }
    Ok(())
}
