use alloy_signer_local::PrivateKeySigner;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

const BASE_URL: &str = "https://api.hyperliquid.xyz";

pub struct HyperliquidClient {
    http: Client,
    base_url: String,
    signer: Option<PrivateKeySigner>,
}

impl HyperliquidClient {
    pub fn new() -> Result<Self> {
        let base_url = std::env::var("HYPERLIQUID_URL").unwrap_or_else(|_| BASE_URL.to_string());
        Ok(Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()?,
            base_url,
            signer: None,
        })
    }

    pub fn new_with_signer() -> Result<Self> {
        let base_url = std::env::var("HYPERLIQUID_URL").unwrap_or_else(|_| BASE_URL.to_string());
        let key = std::env::var("EVM_PRIVATE_KEY")
            .context("EVM_PRIVATE_KEY not set — required for signing")?;
        let signer: PrivateKeySigner = key
            .trim_start_matches("0x")
            .parse()
            .context("failed to parse EVM_PRIVATE_KEY")?;
        Ok(Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()?,
            base_url,
            signer: Some(signer),
        })
    }

    pub async fn info(&self, body: Value) -> Result<Value> {
        let url = format!("{}/info", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Hyperliquid /info request failed")?;
        self.handle_response(resp).await
    }

    pub fn address(&self) -> Result<String> {
        let signer = self
            .signer
            .as_ref()
            .context("EVM_PRIVATE_KEY not set — required for this command")?;
        Ok(format!("{:#x}", signer.address()))
    }

    #[allow(dead_code)]
    pub fn signer(&self) -> Option<&PrivateKeySigner> {
        self.signer.as_ref()
    }

    pub async fn exchange(
        &self,
        action: Value,
        nonce: u64,
        vault_address: Option<&str>,
    ) -> Result<Value> {
        let signer = self
            .signer
            .as_ref()
            .context("EVM_PRIVATE_KEY not set — required for trading commands")?;

        let mainnet = crate::auth::is_mainnet(&self.base_url);
        let signature =
            crate::auth::sign_action(signer, &action, nonce, vault_address, mainnet)?;

        let body = json!({
            "action": action,
            "nonce": nonce,
            "signature": signature,
            "vaultAddress": vault_address,
        });

        let url = format!("{}/exchange", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Hyperliquid /exchange request failed")?;
        self.handle_response(resp).await
    }

    async fn handle_response(&self, resp: reqwest::Response) -> Result<Value> {
        let status = resp.status();
        if status.as_u16() == 429 {
            bail!("Rate limited — retry with backoff");
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("Hyperliquid API error (HTTP {}): {}", status.as_u16(), body);
        }
        let body: Value = resp
            .json()
            .await
            .context("failed to parse Hyperliquid response")?;
        Ok(body)
    }
}
