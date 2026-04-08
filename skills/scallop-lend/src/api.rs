use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::config::SCALLOP_API_URL;

/// Pool address entry from Scallop REST API
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PoolAddressEntry {
    pub coin_name: Option<String>,
    pub symbol: Option<String>,
    pub lending_pool_address: Option<String>,
    pub collateral_pool_address: Option<String>,
    pub borrow_dynamic: Option<String>,
    pub interest_model: Option<String>,
    pub risk_model: Option<String>,
    pub coin_type: Option<String>,
    pub decimals: Option<u8>,
    #[serde(rename = "sCoinSymbol")]
    pub scoin_symbol: Option<String>,
    pub spool: Option<String>,
}

/// Static fallback pool data (from Scallop API /pool/addresses, fetched 2026-04-08)
/// Used when the API is unavailable.
pub fn static_pool_data() -> HashMap<String, PoolAddressEntry> {
    let raw = r#"{
  "usdc":    {"symbol":"USDC",    "lendingPoolAddress":"0xd3be98bf540f7603eeb550c0c0a19dbfc78822f25158b5fa84ebd9609def415f","decimals":6,"coinType":"0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC"},
  "sui":     {"symbol":"SUI",     "lendingPoolAddress":"0x9c9077abf7a29eebce41e33addbcd6f5246a5221dd733e56ea0f00ae1b25c9e8","decimals":9,"coinType":"0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI"},
  "wsol":    {"symbol":"wSOL",    "lendingPoolAddress":"0x985682c42984cdfb03f9ff7d8923344c2fe096b1ae4b82ea17721af19d22a21f","decimals":8,"coinType":"0xb7844e289a8410e50fb3ca48d69eb9cf29e27d223ef90353fe1bd8e27ff8f3f8::coin::COIN"},
  "weth":    {"symbol":"wETH",    "lendingPoolAddress":"0xc8fcdff48efc265740ae0b74aae3faccae9ec00034039a113f3339798035108c","decimals":8,"coinType":"0xaf8cd5edc19c4512f4259f0bee101a40d41ebed738ade5874359610ef8eeced5::coin::COIN"},
  "wbtc":    {"symbol":"wBTC",    "lendingPoolAddress":"0x65cc08a5aca0a0b8d72e1993ded8d145f06dd102fd0d8f285b92934faed564ab","decimals":8,"coinType":"0x027792d9fed7f9844eb4839566001bb6f6cb4804f66aa2da6fe1ee242d896881::coin::COIN"},
  "deep":    {"symbol":"DEEP",    "lendingPoolAddress":"0xf4a67ffb43da1e1c61c049f188f19463ea8dbbf2d5ef4722d6df854ff1b1cc03","decimals":6,"coinType":"0xdeeb7a4662eec9f2f3def03fb937a663dddaa2e215b8078a284d026b7946c270::deep::DEEP"},
  "sca":     {"symbol":"SCA",     "lendingPoolAddress":"0x6fc7d4211fc7018b6c75e7b908b88f2e0536443239804a3d32af547637bd28d7","decimals":9,"coinType":"0x7016aae72cfc67f2fadf55769c0a7dd54291a583b63051a5ed71081cce836ac6::sca::SCA"},
  "hasui":   {"symbol":"haSUI",   "lendingPoolAddress":"0x7ebc607f6bdeb659fb6506cb91c5cc1d47bb365cfd5d2e637ea765346ec84ed4","decimals":9,"coinType":"0xbde4ba4c2e274a60ce15c1cfff9e5c42e41654ac8b6d906a57efa4bd3c29f47d::hasui::HASUI"},
  "afsui":   {"symbol":"afSUI",   "lendingPoolAddress":"0x9b942a24ce390b7f5016d34a0217057bf9487b92aa6d7cc9894271dbbe62471a","decimals":9,"coinType":"0xf325ce1300e8dac124071d3152c5c5ee6174914f8bc2161e88329cf579246efc::afsui::AFSUI"},
  "vsui":    {"symbol":"vSUI",    "lendingPoolAddress":"0xda9257c0731d8822e8a438ebce13415850d705b779c79958dcf2aeb21fcb43d","decimals":9,"coinType":"0x549e8b69270defbfafd4f94e17ec44cdbdd99820b33bda2278dea3b9a32d3f55::cert::CERT"},
  "cetus":   {"symbol":"CETUS",   "lendingPoolAddress":"0xc09858f60e74a1b671635bec4e8a2c84a0ff313eb87f525fba3258e88c6b6282","decimals":9,"coinType":"0x06864a6f921804860930db6ddbe2e16acdf8504495ea7481637a1c8b9a8fe54b::cetus::CETUS"},
  "sbeth":   {"symbol":"sbETH",   "lendingPoolAddress":"0xaa34c938e0394e5186c7dc626ad69be96af2194b23fdc6ac1c63090e399f5ba4","decimals":8,"coinType":"0xd0e89b2af5e4910726fbcd8b8dd37bb79b29e5f83f7491bca830e94f7f226d29::eth::ETH"},
  "sbusdt":  {"symbol":"sbUSDT",  "lendingPoolAddress":"0x958ca02058a7dd8b00e26ed6988f45d7c2834ae2a47ee4c4a8fdedea155f18ca","decimals":6,"coinType":"0x375f70cf2ae4c00bf37117d0c85a2c71545e6ee05c4a5c7d282cd66a4504b068::usdt::USDT"},
  "sbwbtc":  {"symbol":"sbwBTC",  "lendingPoolAddress":"0x5c4fc366c39e0969ddb8912da221cbf298656466f3b58039ff82c5ce64071ad8","decimals":8,"coinType":"0xaafb102dd0902f5055cadecd687fb5b71ca82ef0e0285d90afde828ec58ca96b::btc::BTC"},
  "usdy":    {"symbol":"USDY",    "lendingPoolAddress":"0xd7a8b75ffcd9f22a0108c95ae735b864e117a28d0bf6d596eb4ccd9d6213210d","decimals":6,"coinType":"0x960b531667636f39e85867775f52f6b1f220a058c4de786905bdf761e06a56bb::usdy::USDY"},
  "fdusd":   {"symbol":"FDUSD",   "lendingPoolAddress":"0x4f46051a01f05c3ad9aecf29a771aad5c884e1a1888e08d7709085e3a095bc9c","decimals":6,"coinType":"0xf16e6b723f242ec745dfd7634ad072c42d5c1d9ac9d62a39c381303eaa57693a::fdusd::FDUSD"},
  "ns":      {"symbol":"NS",      "lendingPoolAddress":"0x98491693e99905ce243655f1d2dc86b62d7c9c330985ee71d16760b63601708c","decimals":6,"coinType":"0x5145494a5f5100e645e4b0aa950fa6b68f614e8c59e17bc5ded3495123a79178::ns::NS"},
  "wal":     {"symbol":"WAL",     "lendingPoolAddress":"0xd1dc54a659a5f1b5b26864a1ee0327585c0bd07f066bd3864163db7e73df1209","decimals":9,"coinType":"0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL"},
  "haedal":  {"symbol":"HAEDAL",  "lendingPoolAddress":"0xcc5e913d291e870f3265fb8b260662d84fa2e578dc8b514dfacfbc4562298c0e","decimals":9,"coinType":"0x3a304c7feba2d819ea57c3542d68439ca2c386ba02159c740f7b406e592c62ea::haedal::HAEDAL"},
  "wwal":    {"symbol":"wWAL",    "lendingPoolAddress":"0x34f5d1e516323bd7be77298e2c088fde49302c35bfb330330c0c3d9e45dd6e78","decimals":9,"coinType":"0xb1b0650a8862e30e3f604fd6c5838bc25464b8d3d827fbd58af7cb9685b832bf::wwal::WWAL"},
  "hawal":   {"symbol":"haWAL",   "lendingPoolAddress":"0x8d5188cd7c1fd1b88185c6d2a7eb7243c0861ae4387333f991e0f1096d1a44ff","decimals":9,"coinType":"0x8b4d553839b219c3fd47608a0cc3d5fcc572cb25d41b7df3833208586a8d2470::hawal::HAWAL"},
  "xbtc":    {"symbol":"xBTC",    "lendingPoolAddress":"0x40ff22f6abbf7bdb49cab47ef40ff30f3663a9b83bade9f6749b463cb2274ced","decimals":8,"coinType":"0x876a4b7bce8aeaef60464c11f4026903e9afacab79b9b142686158aa86560b50::xbtc::XBTC"},
  "zwbtc":   {"symbol":"zwBTC",   "lendingPoolAddress":"0xcf5bc04619b3a007966849f54b33927f7b41909e14c0b44cb636c3f82dd6d402","decimals":8,"coinType":"0x0041f9f9344cac094454cd574e333c4fdb132d7bcc9379bcd4aab485b2a63942::wbtc::WBTC"},
  "suiusde": {"symbol":"suiUSDe", "lendingPoolAddress":"0x4ed79138e1833920714e824012ca91a71ac2ac04f9157bcb09a74fa04f11739e","decimals":6,"coinType":"0x41d587e5336f1c86cad50d38a7136db99333bb9bda91cea4ba69115defeb1402::sui_usde::SUI_USDE"},
  "usdsui":  {"symbol":"USDsui",  "lendingPoolAddress":"0xf09633cd0637c5b2aa3a923da932df689afec4dcf96b8030c89bb7443f985955","decimals":6,"coinType":"0x44f838219cf67b058f3b37907b655f226153c18e33dfcd0da559a844fea9b1c1::usdsui::USDSUI"},
  "xaum":    {"symbol":"XAUm",    "lendingPoolAddress":"0xc027dd7c309088f364ed217825a828a745890cb6da090b24331e64282ddb31da","decimals":9,"coinType":"0x9d297676e7a4b771ab023291377b2adfaa4938fb9080b8d12430e4b108b836a9::xaum::XAUM"},
  "sca":     {"symbol":"SCA",     "lendingPoolAddress":"0x6fc7d4211fc7018b6c75e7b908b88f2e0536443239804a3d32af547637bd28d7","decimals":9,"coinType":"0x7016aae72cfc67f2fadf55769c0a7dd54291a583b63051a5ed71081cce836ac6::sca::SCA"},
  "fud":     {"symbol":"FUD",     "lendingPoolAddress":"0xefed2cbe76b344792ac724523c8b2236740d1cea2100d46a0ed0dc760c7f4231","decimals":5,"coinType":"0x76cb819b01abed502bee8a702b4c2d547532c12f25001c9dea795a5e631c26f1::fud::FUD"},
  "blub":    {"symbol":"BLUB",    "lendingPoolAddress":"0x4dede1d8eda98647c3fc9838e94a890b73ca37a20764087eb78ba0473edea1a5","decimals":2,"coinType":"0xfa7ac3951fdca92c5200d468d31a365eb03b2be9936fde615e69f0c1274ad3a0::BLUB::BLUB"},
  "wusdc":   {"symbol":"wUSDC",   "lendingPoolAddress":"0x2f4df5e1368fbbdaa5c712d28b837b3d41c2d3872979ccededcdfdac55ff8a93","decimals":6,"coinType":"0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN"},
  "wusdt":   {"symbol":"wUSDT",   "lendingPoolAddress":"0xfbc056f126dd35adc1f8fe985e2cedc8010e687e8e851e1c5b99fdf63cd1c879","decimals":6,"coinType":"0xc060006111016b8a020ad5b33834984a437aaa7d3c74c18e09a95d48aceab08c::coin::COIN"},
  "musd":    {"symbol":"MUSD",    "lendingPoolAddress":"0xa77bda4fdb2b1c1ba4fe51fc3ef0d69c3484a81e25d0f3bd43040a39f8f2b25a","decimals":9,"coinType":"0xe44df51c0b21a27ab915fa1fe2ca610cd3eaa6d9666fe5e62b988bf7f0bd8722::musd::MUSD"},
  "wapt":    {"symbol":"wAPT",    "lendingPoolAddress":"0xca8c14a24e0c32b198eaf479a3317461e3cc339097ce88eaf296a15df8dcfdf5","decimals":8,"coinType":"0x3a5143bb1196e3bcdfab6203d1683ae29edd26294fc8bfeafe4aaa9d2704df37::coin::COIN"}
}"#;
    let map: HashMap<String, serde_json::Value> = serde_json::from_str(raw).unwrap_or_default();
    let mut result = HashMap::new();
    for (k, v) in map {
        if let Ok(entry) = serde_json::from_value::<PoolAddressEntry>(v) {
            result.insert(k, entry);
        }
    }
    result
}

/// Fetch pool addresses from Scallop API, falling back to static data
pub async fn fetch_pool_addresses(client: &Client) -> Result<HashMap<String, PoolAddressEntry>> {
    let url = format!("{}/pool/addresses", SCALLOP_API_URL);

    match client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(response) => {
            match response.bytes().await {
                Ok(bytes) if !bytes.is_empty() => {
                    // Attempt gzip decompression
                    let data: Vec<u8> = if bytes.starts_with(&[0x1f, 0x8b]) {
                        use std::io::Read;
                        let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
                        let mut decompressed = Vec::new();
                        if decoder.read_to_end(&mut decompressed).is_ok() {
                            decompressed
                        } else {
                            bytes.to_vec()
                        }
                    } else {
                        bytes.to_vec()
                    };

                    // Try parsing JSON
                    if let Ok(resp) = serde_json::from_slice::<Value>(&data) {
                        if resp.is_object() {
                            let mut result: HashMap<String, PoolAddressEntry> = HashMap::new();
                            if let Some(obj) = resp.as_object() {
                                for (key, val) in obj {
                                    if let Ok(entry) =
                                        serde_json::from_value::<PoolAddressEntry>(val.clone())
                                    {
                                        result.insert(key.clone(), entry);
                                    }
                                }
                            }
                            if !result.is_empty() {
                                return Ok(result);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Err(_) => {}
    }

    // Fallback to embedded static data
    Ok(static_pool_data())
}
