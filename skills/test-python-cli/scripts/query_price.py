#!/usr/bin/env python3
"""E2E test - Query ETH price via onchainos."""
import subprocess
import sys

def main():
    if len(sys.argv) > 1 and sys.argv[1] == "--help":
        print("test-python-cli v1.0.0")
        print("Usage: python3 query_price.py --query eth-price")
        print("Queries ETH price via onchainos token price ETH")
        return

    if len(sys.argv) > 2 and sys.argv[1] == "--query" and sys.argv[2] == "eth-price":
        print("Querying ETH price via onchainos...")
        try:
            result = subprocess.run(
                ["onchainos", "token", "price-info", "--address", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "--chain", "ethereum"],
                capture_output=True, text=True
            )
            print(result.stdout)
            if result.returncode != 0:
                print(f"Error: {result.stderr}", file=sys.stderr)
        except FileNotFoundError:
            print("Error: onchainos not installed. Run pre-flight install.", file=sys.stderr)
            sys.exit(1)
    else:
        print("test-python-cli v1.0.0 - E2E test Python CLI")
        print("Run with --help for usage")

if __name__ == "__main__":
    main()
