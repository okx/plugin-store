// Shared harness for the autotrade-gate integration tests.
//
// Two fakes, no extra dependencies:
//   1. mock_onchainos.sh — intercepts every onchainos subprocess call (wallet lookup,
//      grant check) and records invocations to a log file.
//   2. MockHlApi — a threaded TcpListener impersonating api.hyperliquid.xyz, dispatching
//      on the `type` field of each /info POST body.
//
// The gate under test sits behind read-only market/position lookups, so those lookups
// have to resolve before any gate assertion can run at all.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const TEST_WALLET: &str = "0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF";
pub const TEST_COIN: &str = "BTC";
pub const TEST_MID: &str = "100000";
pub const TEST_SZ_DECIMALS: u64 = 5;
/// Substring the mock's `crash` mode writes to stderr; the plugin must never echo it.
pub const GRANT_FILE_MARKER: &str = "job-secret.json";

pub fn binary() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("hyperliquid-plugin")
}

pub fn mock_onchainos_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mock_onchainos.sh")
}

// ── Mock Hyperliquid API ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MockApiConfig {
    /// Signed position size for clearinghouseState; None = no open position.
    pub position_szi: Option<String>,
    pub mid: String,
    pub sz_decimals: u64,
    pub only_isolated: bool,
    pub perp_withdrawable: String,
}

impl Default for MockApiConfig {
    fn default() -> Self {
        Self {
            position_szi: Some("0.01".to_string()),
            mid: TEST_MID.to_string(),
            sz_decimals: TEST_SZ_DECIMALS,
            only_isolated: false,
            perp_withdrawable: "100000.0".to_string(),
        }
    }
}

pub struct MockHlApi {
    pub base: String,
    stop: Arc<AtomicBool>,
}

impl Drop for MockHlApi {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Unblock the accept loop so the thread can observe the stop flag.
        let _ = TcpStream::connect(self.base.trim_start_matches("http://"));
    }
}

pub fn start_mock_api(cfg: MockApiConfig) -> MockHlApi {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock api");
    let addr = listener.local_addr().expect("local_addr");
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if stop_thread.load(Ordering::SeqCst) {
                break;
            }
            match stream {
                Ok(s) => {
                    let cfg = cfg.clone();
                    std::thread::spawn(move || handle_conn(s, cfg));
                }
                Err(_) => break,
            }
        }
    });

    MockHlApi {
        base: format!("http://{}", addr),
        stop,
    }
}

fn handle_conn(mut stream: TcpStream, cfg: MockApiConfig) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let is_exchange = request_line.contains("/exchange");

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return;
        }
        if line == "\r\n" || line.is_empty() {
            break;
        }
        if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 && reader.read_exact(&mut body).is_err() {
        return;
    }
    let body = String::from_utf8_lossy(&body).to_string();

    let payload = if is_exchange {
        exchange_response()
    } else {
        info_response(&body, &cfg)
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn info_response(body: &str, cfg: &MockApiConfig) -> String {
    let has = |t: &str| body.contains(&format!("\"{}\"", t));

    // The order path reads an Arbitrum USDC balance before authorization and treats a
    // failure there as fatal, so the same fake also speaks JSON-RPC.
    if has("eth_call") || has("eth_getBalance") {
        // 100,000 USDC at 6 decimals.
        return r#"{"jsonrpc":"2.0","id":1,"result":"0x0000000000000000000000000000000000000000000000000000174876e800"}"#
            .to_string();
    }
    if has("perpDexs") {
        return "[]".to_string();
    }
    if has("metaAndAssetCtxs") {
        return format!(
            r#"[{{"universe":[{{"name":"{coin}","szDecimals":{sz},"onlyIsolated":{isolated},"maxLeverage":50}}]}},[{{"markPx":"{mid}","prevDayPx":"{mid}","oraclePx":"{mid}"}}]]"#,
            coin = TEST_COIN,
            sz = cfg.sz_decimals,
            mid = cfg.mid,
            isolated = cfg.only_isolated,
        );
    }
    if has("meta") && !has("spotMeta") {
        return format!(
            r#"{{"universe":[{{"name":"{coin}","szDecimals":{sz},"onlyIsolated":{isolated},"maxLeverage":50}}]}}"#,
            coin = TEST_COIN,
            sz = cfg.sz_decimals,
            isolated = cfg.only_isolated,
        );
    }
    if has("allMids") {
        return format!(r#"{{"{coin}":"{mid}"}}"#, coin = TEST_COIN, mid = cfg.mid);
    }
    if has("spotClearinghouseState") {
        return r#"{"balances":[{"coin":"USDC","total":"100000.0","hold":"0.0"}]}"#.to_string();
    }
    if has("clearinghouseState") {
        let positions = match &cfg.position_szi {
            Some(szi) => format!(
                r#"[{{"position":{{"coin":"{coin}","szi":"{szi}","entryPx":"{mid}","leverage":{{"type":"cross","value":3}}}},"type":"oneWay"}}]"#,
                coin = TEST_COIN,
                szi = szi,
                mid = cfg.mid
            ),
            None => "[]".to_string(),
        };
        return format!(
            r#"{{"assetPositions":{positions},"marginSummary":{{"accountValue":"100000.0","totalMarginUsed":"0.0","totalNtlPos":"0.0"}},"crossMarginSummary":{{"accountValue":"100000.0"}},"withdrawable":"{withdrawable}"}}"#,
            positions = positions,
            withdrawable = cfg.perp_withdrawable,
        );
    }
    if has("openOrders") {
        return "[]".to_string();
    }
    "{}".to_string()
}

fn exchange_response() -> String {
    r#"{"status":"ok","response":{"type":"order","data":{"statuses":[{"filled":{"totalSz":"0.01","avgPx":"100000","oid":9876543}}]}}}"#
        .to_string()
}

// ── Invocation log ────────────────────────────────────────────────────────────

pub struct CallLog {
    path: PathBuf,
}

impl CallLog {
    pub fn new(tag: &str) -> Self {
        // A unique path per instance plus an explicit truncate: file removal can fail
        // silently in a sandboxed temp dir, which would let one case's calls bleed into
        // the next case's assertions.
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        // Under the crate's target dir, not the system temp dir: the latter is not
        // writable in the sandboxed dev environment.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-logs");
        std::fs::create_dir_all(&dir).expect("create call-log dir");
        let path = dir.join(format!(
            "calls-{}-{}-{}.jsonl",
            tag,
            std::process::id(),
            seq
        ));
        std::fs::write(&path, b"").expect("create call log");
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn lines(&self) -> Vec<String> {
        std::fs::read_to_string(&self.path)
            .unwrap_or_default()
            .lines()
            .map(|l| l.to_string())
            .collect()
    }

    pub fn grant_checks(&self) -> Vec<String> {
        self.lines()
            .into_iter()
            .filter(|l| l.contains("autotrade-grant-check"))
            .collect()
    }

    pub fn signing_calls(&self) -> Vec<String> {
        self.lines()
            .into_iter()
            .filter(|l| l.contains("sign-message") || l.contains("contract-call"))
            .collect()
    }
}

impl Drop for CallLog {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ── Command runner ────────────────────────────────────────────────────────────

pub struct Run {
    pub output: Output,
}

impl Run {
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }
    pub fn json(&self) -> serde_json::Value {
        // The commands print a preview object and then the result; take the last JSON value.
        let out = self.stdout();
        let mut decoder = serde_json::Deserializer::from_str(&out).into_iter::<serde_json::Value>();
        let mut last = serde_json::Value::Null;
        while let Some(Ok(v)) = decoder.next() {
            last = v;
        }
        last
    }
    pub fn first_json(&self) -> serde_json::Value {
        let out = self.stdout();
        serde_json::Deserializer::from_str(&out)
            .into_iter::<serde_json::Value>()
            .next()
            .and_then(|r| r.ok())
            .unwrap_or(serde_json::Value::Null)
    }
}

/// Invoke the plugin with the mock onchainos and mock API wired in.
pub fn run_plugin(
    args: &[&str],
    api: &MockHlApi,
    log: &CallLog,
    grant_result: Option<&str>,
    extra_env: &[(&str, &str)],
) -> Run {
    let mut cmd = Command::new(binary());
    cmd.args(args)
        .env("HYPERLIQUID_TEST_MODE", "1")
        .env("HYPERLIQUID_TEST_API_BASE", &api.base)
        .env("HYPERLIQUID_TEST_ARBITRUM_RPC", &api.base)
        .env("HYPERLIQUID_ONCHAINOS_BIN", mock_onchainos_path())
        .env("MOCK_ONCHAINOS_CALL_LOG", log.path())
        .env("MOCK_ONCHAINOS_WALLET", TEST_WALLET);
    if let Some(g) = grant_result {
        cmd.env("MOCK_ONCHAINOS_GRANT_RESULT", g);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("spawn plugin binary");
    Run { output }
}
