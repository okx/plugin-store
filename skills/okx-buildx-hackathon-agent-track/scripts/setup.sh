#!/usr/bin/env bash
# OKX Build X AI Hackathon — fetch reference docs
# Run: bash setup.sh

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
REF="$DIR/reference"

mkdir -p "$REF"

echo "Fetching reference docs..."

curl -sf "https://www.moltbook.com/skill.md" -o "$REF/moltbook-skill.md" && \
  echo "  ✓ moltbook-skill.md" || echo "  ✗ moltbook-skill.md (failed)"

curl -sf "https://web3.okx.com/llms.txt" -o "$REF/onchainos-llms.txt" && \
  echo "  ✓ onchainos-llms.txt" || echo "  ✗ onchainos-llms.txt (failed)"

curl -sf "https://web3.okx.com/llms-full.txt" -o "$REF/onchainos-llms-full.txt" && \
  echo "  ✓ onchainos-llms-full.txt" || echo "  ✗ onchainos-llms-full.txt (failed)"

curl -sf "https://docs.uniswap.org/v4-llms.txt" -o "$REF/uniswap-v4-llms.txt" && \
  echo "  ✓ uniswap-v4-llms.txt" || echo "  ✗ uniswap-v4-llms.txt (failed)"

echo ""
echo "Done. Reference docs saved to: $REF/"
echo "Re-run anytime to update."
