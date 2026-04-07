use crate::rpc;
use clap::Args;

#[derive(Args)]
pub struct StatusArgs {
    /// Bridge request ID (from quote or bridge command)
    #[arg(long)]
    pub request_id: String,
}

pub fn run(args: StatusArgs) -> anyhow::Result<()> {
    println!("=== Relay Bridge Status ===");
    println!("Request ID: {}", args.request_id);
    println!();

    let result = rpc::get_status(&args.request_id)?;

    let status = result["status"].as_str().unwrap_or("unknown");

    let status_msg = match status {
        "waiting" => "Waiting — transaction received, awaiting confirmation",
        "pending" => "Pending — bridge in progress",
        "success" => "Success — bridge completed",
        "failed"  => "Failed — bridge failed",
        "refunded" => "Refunded — transaction was refunded",
        "unknown" => "Unknown — request not found or not yet indexed",
        other => other,
    };

    println!("Status: {} — {}", status, status_msg);

    // Display any additional fields
    if let Some(obj) = result.as_object() {
        for (k, v) in obj {
            if k != "status" {
                println!("  {}: {}", k, v);
            }
        }
    }

    Ok(())
}
