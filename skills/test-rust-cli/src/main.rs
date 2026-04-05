use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--query" && args.get(2).map(|s| s.as_str()) == Some("eth-price") {
        println!("Querying ETH price via onchainos...");
        let output = Command::new("onchainos")
            .args(["token", "price-info", "--address", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "--chain", "ethereum"])
            .output();
        match output {
            Ok(o) => print!("{}", String::from_utf8_lossy(&o.stdout)),
            Err(e) => eprintln!("Error: {}", e),
        }
    } else if args.len() > 1 && args[1] == "--help" {
        println!("test-rust-cli v1.0.0");
        println!("Usage: test-rust-cli --query eth-price");
        println!("Queries ETH price via onchainos token price ETH");
    } else {
        println!("test-rust-cli v1.0.0 - E2E test Rust CLI");
        println!("Run with --help for usage");
    }
}
