/// `fourmeme-plugin config` — fetch four.meme public sys/config.

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct ConfigArgs {}

pub async fn run(_args: ConfigArgs) -> Result<()> {
    match crate::api::fetch_public_config().await {
        Ok(data) => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": data,
            }))?);
            Ok(())
        }
        Err(e) => {
            println!("{}", super::error_response(&e, Some("config"), None));
            Ok(())
        }
    }
}
