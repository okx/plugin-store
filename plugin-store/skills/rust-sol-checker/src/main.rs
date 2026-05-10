use std::io::Write;
use std::process::{Command, exit};

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-h" | "--help" => {
                println!("Usage: rust-sol-checker");
                println!("  Prints SOL price JSON from `onchainos token price-info`.");
                return;
            }
            "-v" | "--version" => {
                println!("rust-sol-checker {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                exit(2);
            }
        }
    }

    let output = match Command::new("onchainos")
        .args([
            "token",
            "price-info",
            "--address",
            SOL_MINT,
            "--chain",
            "solana",
        ])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("ERROR: failed to invoke onchainos: {}", e);
            exit(1);
        }
    };

    if !output.status.success() {
        eprintln!(
            "onchainos exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        exit(1);
    }

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(&output.stdout).ok();
}
