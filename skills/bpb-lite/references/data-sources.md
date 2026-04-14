# Data Sources and Trust Matrix

BPB Lite is the public-facing intelligence layer for Based Pings Bot.

The goal is to explain why a token may deserve a ping by combining market movement, timing, structure, attribution, lock context, and wallet behavior into one compact read.

## Source posture

For the public hackathon story, BPB Lite is presented as **OKX OnchainOS-first**.

That does not mean every field comes from one endpoint. It means OKX OnchainOS is the headline intelligence layer, while other sources strengthen attribution, verification, and operator follow-up.

## Trust matrix

| Source | Role | Typical fields | Confidence impact | Notes |
|---|---|---|---|---|
| OKX OnchainOS | Primary intelligence layer | market structure, token context, trend framing, momentum context | High | Main public-facing signal story |
| DexScreener | Pair and venue visibility | liquidity, volume, market cap, pair URLs, age | Medium to High | Useful for fast venue and market reads |
| Printr preview API | Direct launch attribution | launch source, token identity | High | Strongest public source for Printr-native confirmation |
| Helius | Solana-native verification | wallet context, holder verification, token structure support | Medium to High | Helps support smart-money and structure reads |
| Streamflow | Lock context | lock status, unlock timing | Medium | Useful for token-quality context |
| Solscan | Explorer follow-up | token page, transaction inspection | Low to Medium | Operator follow-up layer |
| X search | Discovery follow-up | contract search, social lookup | Low | Context only, not proof |

## Public capability story

BPB can honestly say it is built around:

- volume injections and timing windows
- momentum recognition
- liquidity and holder structure
- smart-wallet context from a large tracked wallet set
- lock-aware token quality checks
- platform-aware interpretation across Pumpfun, Bonk, LaunchLab, Printr, Meteora, DBC, Raydium, and RISE

## Attribution policy

- Use **Printr** when direct token metadata confirms it.
- Use **Meteora** when public pair or rail evidence points to it.
- Use **RISE** as a platform context when evidence supports it.
- Separate launch-source claims from downstream liquidity-venue claims when both are relevant.
- If the read is incomplete, describe the evidence as partial instead of forcing a brittle label.

## Reliability notes

- Not every field is always available from public sources.
- Confidence should fall when attribution is indirect or market structure is sparse.
- The scanner should degrade gracefully and still produce a readable signal-oriented result.

## Helpful references

- https://web3.okx.com/llms.txt
- https://web3.okx.com/onchainos
- https://github.com/okx/plugin-store/tree/main/skills/okx-buildx-hackathon-agent-track
