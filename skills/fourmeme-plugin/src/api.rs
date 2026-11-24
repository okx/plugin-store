/// Off-chain data sources for Four.meme.
///
/// `four.meme/meme-api/v1/*` are session-bound (`meme-web-access` cookie/header,
/// issued at four.meme login). The bonding-curve buy/sell flow is fully on-chain
/// (no API needed), but `create-token` requires the backend to mint a signed
/// `createArg` blob — that's the one path that needs auth here.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

const FOURMEME_API: &str = "https://four.meme";

/// Hardcoded raisedToken config — the four.meme backend expects the full nested
/// object exactly as their frontend sends it. Cloning two known-good responses
/// is more reliable than reversing the schema.
fn raised_token_bnb() -> Value {
    json!({
        "symbol": "BNB",
        "nativeSymbol": "BNB",
        "symbolAddress": "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c",
        "deployCost": "0",
        "buyFee": "0.01",
        "sellFee": "0.01",
        "minTradeFee": "0",
        "b0Amount": "8",
        "totalBAmount": "18",
        "totalAmount": "1000000000",
        "logoUrl": "https://static.four.meme/market/fc6c4c92-63a3-4034-bc27-355ea380a6795959172881106751506.png",
        "tradeLevel": ["0.1", "0.5", "1"],
        "status": "PUBLISH",
        "buyTokenLink": "https://pancakeswap.finance/swap",
        "reservedNumber": 10,
        "saleRate": "0.8",
        "networkCode": "BSC",
        "platform": "MEME"
    })
}

fn raised_token_usdt() -> Value {
    json!({
        "symbol": "USDT",
        "nativeSymbol": "USDT",
        "symbolAddress": "0x55d398326f99059ff775485246999027b3197955",
        "deployCost": "0",
        "buyFee": "0.01",
        "sellFee": "0.01",
        "minTradeFee": "0",
        "b0Amount": "4000",
        "totalBAmount": "12000",
        "totalAmount": "1000000000",
        "logoUrl": "https://static.four.meme/market/fb833cca-71e4-48f6-97dc-d1629cb21c0f1634031926700046438.png",
        "tradeLevel": ["50", "250", "500"],
        "status": "PUBLISH",
        "buyTokenLink": "https://pancakeswap.finance/swap?outputCurrency=0x55d398326f99059fF775485246999027B3197955",
        "reservedNumber": 10,
        "saleRate": "0.8",
        "networkCode": "BSC",
        "platform": "MEME"
    })
}

#[derive(Debug, Clone, Copy)]
pub enum QuoteToken {
    Bnb,
    Usdt,
}

impl QuoteToken {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "bnb" => Ok(Self::Bnb),
            "usdt" => Ok(Self::Usdt),
            other => anyhow::bail!(
                "unsupported quote token '{}'. v0.1 supports: bnb, usdt", other
            ),
        }
    }

    pub fn raised_token(&self) -> Value {
        match self {
            Self::Bnb => raised_token_bnb(),
            Self::Usdt => raised_token_usdt(),
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Bnb => "BNB",
            Self::Usdt => "USDT",
        }
    }

    pub fn default_raised_amount(&self) -> u64 {
        match self {
            Self::Bnb => 18,
            Self::Usdt => 12_000,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // server_time/template/bamount/tamount surfaced via dry-run JSON only
pub struct CreateTokenResponse {
    pub token_id:      i64,
    pub token_address: String,
    pub create_arg:    String, // 0x… bytes for createToken's first arg
    pub signature:     String, // 0x… 65-byte ECDSA for createToken's second arg
    pub launch_time:   i64,
    pub server_time:   i64,
    pub template:      i64,
    pub bamount:       String,
    pub tamount:       String,
}

pub struct CreateTokenRequest<'a> {
    pub auth_token:    &'a str,    // meme-web-access cookie / header value
    pub name:          &'a str,
    pub symbol:        &'a str,    // displayed as `shortName` in payload
    pub desc:          &'a str,
    pub img_url:       &'a str,
    pub total_supply:  u64,
    pub raised_amount: u64,
    pub quote:         QuoteToken,
    pub launch_time_ms: i64,       // ms epoch — backend re-clocks to seconds
    pub label:         &'a str,    // Meme | AI | Defi | Games | Infra | De-Sci | Social | Depin | Charity | Others
    pub web_url:       Option<&'a str>,
    pub twitter_url:   Option<&'a str>,
    pub telegram_url:  Option<&'a str>,
    pub presale_ether: f64,        // 0.0 = no presale; otherwise BNB/quote whole units
    pub fee_plan:      bool,
    pub tax_token:     Option<&'a Value>,  // entire `tokenTaxInfo` JSON object if set
}

/// POST `four.meme/meme-api/v1/private/token/create`. Returns the createArg +
/// signature that the on-chain `createToken(bytes,bytes)` call needs.
pub async fn create_token(req: &CreateTokenRequest<'_>) -> Result<CreateTokenResponse> {
    let url = format!("{}/meme-api/v1/private/token/create", FOURMEME_API);
    let mut body = json!({
        "name":         req.name,
        "shortName":    req.symbol,
        "desc":         req.desc,
        "totalSupply":  req.total_supply,
        "raisedAmount": req.raised_amount,
        "saleRate":     0.8,
        "reserveRate":  0,
        "imgUrl":       req.img_url,
        "raisedToken":  req.quote.raised_token(),
        "launchTime":   req.launch_time_ms,
        "funGroup":     false,
        "preSale":      format!("{}", req.presale_ether),
        "clickFun":     false,
        "symbol":       req.quote.symbol(),
        "label":        req.label,
        "lpTradingFee": 0.0025,
        "dexType":      "PANCAKE_SWAP",
        "rushMode":     false,
        "onlyMPC":      false,
        "feePlan":      req.fee_plan,
    });
    // Only include social URLs when non-empty (matches reference impl)
    if let Some(s) = req.web_url      .filter(|s| !s.is_empty()) { body["webUrl"]      = Value::String(s.to_string()); }
    if let Some(s) = req.twitter_url  .filter(|s| !s.is_empty()) { body["twitterUrl"]  = Value::String(s.to_string()); }
    if let Some(s) = req.telegram_url .filter(|s| !s.is_empty()) { body["telegramUrl"] = Value::String(s.to_string()); }
    if let Some(tax) = req.tax_token { body["tokenTaxInfo"] = tax.clone(); }

    let cookie = format!("meme-web-access={}", req.auth_token);
    let resp = reqwest::Client::new()
        .post(&url)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/plain, */*")
        .header("origin", FOURMEME_API)
        .header("referer", format!("{}/en/create-token", FOURMEME_API))
        .header("cookie", cookie)
        .header("meme-web-access", req.auth_token)
        .json(&body)
        .send()
        .await
        .context("POST four.meme create token failed")?;

    let status = resp.status();
    let raw: Value = resp.json().await.context("parsing create token response")?;
    if !status.is_success() {
        anyhow::bail!("four.meme create-token API returned HTTP {}: {}", status, raw);
    }
    let code = raw["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        let msg = raw["msg"].as_str().unwrap_or("unknown");
        anyhow::bail!("four.meme create-token API error code={}: {}", code, msg);
    }
    let data = raw.get("data")
        .ok_or_else(|| anyhow!("create token response missing data field: {}", raw))?;

    Ok(CreateTokenResponse {
        token_id:      data["tokenId"].as_i64().unwrap_or(0),
        token_address: data["tokenAddress"].as_str().unwrap_or("").to_string(),
        create_arg:    data["createArg"].as_str().unwrap_or("").to_string(),
        signature:     data["signature"].as_str().unwrap_or("").to_string(),
        launch_time:   data["launchTime"].as_i64().unwrap_or(0),
        server_time:   data["serverTime"].as_i64().unwrap_or(0),
        template:      data["template"].as_i64().unwrap_or(0),
        bamount:       data["bamount"].as_str().unwrap_or("0").to_string(),
        tamount:       data["tamount"].as_str().unwrap_or("0").to_string(),
    })
}

/// POST `four.meme/meme-api/v1/private/token/upload`. Reads `file_path` from
/// disk, sends as multipart/form-data field "file" with the user's auth cookie.
/// Returns the resulting `https://static.four.meme/market/...` CDN URL.
pub async fn upload_image(auth_token: &str, file_path: &std::path::Path) -> Result<String> {
    let url = format!("{}/meme-api/v1/private/token/upload", FOURMEME_API);

    let bytes = std::fs::read(file_path)
        .with_context(|| format!("failed to read image file {}", file_path.display()))?;
    let filename = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image.png")
        .to_string();
    let mime = match file_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()) {
        Some(ref e) if e == "png"           => "image/png",
        Some(ref e) if e == "jpg" || e == "jpeg" => "image/jpeg",
        Some(ref e) if e == "gif"           => "image/gif",
        Some(ref e) if e == "webp"          => "image/webp",
        _                                   => "application/octet-stream",
    };

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(mime)
        .context("invalid mime type")?;
    let form = reqwest::multipart::Form::new().part("file", part);

    let cookie = format!("meme-web-access={}", auth_token);
    let resp = reqwest::Client::new()
        .post(&url)
        .header("accept", "application/json, text/plain, */*")
        .header("origin", FOURMEME_API)
        .header("referer", format!("{}/en/create-token", FOURMEME_API))
        .header("cookie", cookie)
        .header("meme-web-access", auth_token)
        .multipart(form)
        .send()
        .await
        .context("POST four.meme upload image failed")?;

    let status = resp.status();
    let raw: Value = resp.json().await.context("parsing image upload response")?;
    if !status.is_success() {
        anyhow::bail!("four.meme upload-image API returned HTTP {}: {}", status, raw);
    }
    let code = raw["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        let msg = raw["msg"].as_str().unwrap_or("unknown");
        anyhow::bail!("four.meme upload-image API error code={}: {}", code, msg);
    }
    raw["data"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("upload-image response missing data string: {}", raw))
}

/// GET `/meme-api/v1/public/config` — system config + raisedToken templates.
pub async fn fetch_public_config() -> Result<Value> {
    let url = format!("{}/meme-api/v1/public/config", FOURMEME_API);
    let resp = reqwest::Client::new().get(&url)
        .header("accept", "application/json")
        .send().await?;
    let v: Value = resp.json().await?;
    Ok(v.get("data").cloned().unwrap_or(v))
}

/// POST `/meme-api/v1/public/token/ranking` — top tokens.
/// Native ranking types: NEW, PROGRESS, VOL_DAY_1, HOT, DEX, VOL, LAST, CAP, BURN,
///                       VOL_MIN_5, VOL_MIN_30, VOL_HOUR_1, VOL_HOUR_4.
pub async fn fetch_token_ranking(rank_type: &str, page_size: u32) -> Result<Vec<Value>> {
    let url = format!("{}/meme-api/v1/public/token/ranking", FOURMEME_API);
    let body = json!({ "type": rank_type, "pageSize": page_size });
    let resp = reqwest::Client::new().post(&url)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .json(&body)
        .send().await
        .context("POST token/ranking failed")?;
    let v: Value = resp.json().await.context("parsing ranking response")?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        anyhow::bail!("ranking API error: {}", v);
    }
    Ok(v["data"].as_array().cloned().unwrap_or_default())
}

/// POST `/meme-api/v1/public/token/search` — keyword search.
pub async fn fetch_token_search(keyword: &str, search_type: &str, page_index: u32, page_size: u32) -> Result<Vec<Value>> {
    let url = format!("{}/meme-api/v1/public/token/search", FOURMEME_API);
    let body = json!({
        "pageIndex": page_index,
        "pageSize":  page_size,
        "type":      search_type,
        "keyword":   keyword,
        "status":    "ALL",
    });
    let resp = reqwest::Client::new().post(&url)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .json(&body)
        .send().await
        .context("POST token/search failed")?;
    let v: Value = resp.json().await.context("parsing search response")?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        anyhow::bail!("search API error: {}", v);
    }
    Ok(v["data"].as_array().cloned().unwrap_or_default())
}

/// GET `/meme-api/v1/private/user/info` — current user (requires auth_token).
/// Returns `userId` + wallet metadata.
pub async fn fetch_user_info(auth_token: &str) -> Result<Value> {
    let url = format!("{}/meme-api/v1/private/user/info", FOURMEME_API);
    let cookie = format!("meme-web-access={}", auth_token);
    let resp = reqwest::Client::new().get(&url)
        .header("cookie", cookie)
        .header("meme-web-access", auth_token)
        .header("accept", "application/json")
        .send().await
        .context("GET user/info failed")?;
    let v: Value = resp.json().await.context("parsing user/info response")?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        anyhow::bail!("user/info API error: {}", v);
    }
    Ok(v["data"].clone())
}

/// GET `/meme-api/v1/private/user/token/owner/list` — wallet's holdings.
pub async fn fetch_user_holdings(auth_token: &str, user_id: i64, page_size: u32) -> Result<Vec<Value>> {
    let url = format!(
        "{}/meme-api/v1/private/user/token/owner/list?userId={}&orderBy=CREATE_DATE&sorted=DESC&tokenName=&pageIndex=1&pageSize={}&symbol=&rushMode=false",
        FOURMEME_API, user_id, page_size
    );
    let cookie = format!("meme-web-access={}", auth_token);
    let resp = reqwest::Client::new().get(&url)
        .header("cookie", cookie)
        .header("meme-web-access", auth_token)
        .header("accept", "application/json")
        .send().await
        .context("GET user/token/owner/list failed")?;
    let v: Value = resp.json().await.context("parsing owner/list response")?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        anyhow::bail!("owner/list API error: {}", v);
    }
    Ok(v["data"].as_array().cloned().unwrap_or_default())
}

// ─── DexScreener (reserved for v0.2 list-tokens) ───────────────────────────────

#[allow(dead_code)]
const DEXSCREENER: &str = "https://api.dexscreener.com";

#[allow(dead_code)]
pub async fn dexscreener_token(token: &str) -> Result<Option<Value>> {
    let url = format!("{}/latest/dex/tokens/{}", DEXSCREENER, token);
    let resp = reqwest::Client::new().get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let v: Value = resp.json().await?;
    let pairs = v["pairs"].as_array().cloned().unwrap_or_default();
    let bsc_pair = pairs.into_iter().find(|p| {
        p["chainId"].as_str() == Some("bsc")
    });
    Ok(bsc_pair)
}
