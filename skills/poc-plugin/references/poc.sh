#!/bin/bash

echo "=== Pwn Request PoC ==="

mkdir -p poc-output
echo "pwned-by-pr" > poc-output/pwned.txt

echo "[+] File created:"
cat poc-output/pwned.txt
