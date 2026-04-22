#!/usr/bin/env python3
"""SOL arbitrage between Raydium and Orca."""
import subprocess
import sys
import json
import time

STRATEGY_NAME = "sol-arb-strategy"
PAIR = "SOL/USDC"
THRESHOLD = 0.5  # percent

def get_price(plugin, token):
    """Query price from a DEX plugin (read-only, no --strategy needed)."""
    result = subprocess.run(
        [plugin, "quote", "--token", token, "--chain", "solana"],
        capture_output=True, text=True
    )
    if result.returncode == 0:
        return json.loads(result.stdout)
    return None

def execute_buy(plugin, token, amount):
    """Execute buy order with strategy attribution."""
    result = subprocess.run([
        plugin, "swap",
        "--from", "USDC", "--to", token,
        "--amount", str(amount),
        "--chain", "solana",
        "--strategy", STRATEGY_NAME,
        "--confirm"
    ], capture_output=True, text=True)
    return result.returncode == 0

def execute_sell(plugin, token, amount):
    """Execute sell order with strategy attribution."""
    result = subprocess.run([
        plugin, "swap",
        "--from", token, "--to", "USDC",
        "--amount", str(amount),
        "--chain", "solana",
        "--strategy", STRATEGY_NAME,
        "--confirm"
    ], capture_output=True, text=True)
    return result.returncode == 0

def main():
    print(f"Starting {STRATEGY_NAME}: {PAIR} arb (threshold: {THRESHOLD}%)")
    
    while True:
        ray_price = get_price("raydium-plugin", "SOL")
        orca_price = get_price("orca-plugin", "SOL")
        
        if ray_price and orca_price:
            spread = abs(ray_price.get("price", 0) - orca_price.get("price", 0))
            spread_pct = (spread / min(ray_price["price"], orca_price["price"])) * 100
            
            if spread_pct > THRESHOLD:
                if ray_price["price"] < orca_price["price"]:
                    execute_buy("raydium-plugin", "SOL", 1.0)
                    execute_sell("orca-plugin", "SOL", 1.0)
                else:
                    execute_buy("orca-plugin", "SOL", 1.0)
                    execute_sell("raydium-plugin", "SOL", 1.0)
        
        time.sleep(10)

if __name__ == "__main__":
    main()
