use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--help" {
        println!("rust-cli-inspector v1.1.0");
        println!("Usage: rust-cli-inspector [--query eth-price]");
        println!("Queries ETH price via onchainos token price-info");
    } else {
        println!("rust-cli-inspector v1.1.0 - Querying ETH price via onchainos...");
        let output = Command::new("onchainos")
            .args(["token", "price-info", "--address", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "--chain", "ethereum"])
            .output();
        match output {
            Ok(o) => print!("{}", String::from_utf8_lossy(&o.stdout)),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
