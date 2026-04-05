# top-rank-tokens-sniper
Automated OKX ranking leaderboard sniper that monitors the Solana 1h gainers Top 20 every 10 seconds, applies momentum scoring and 3-level safety checks, and auto-exits when tokens drop off the leaderboard.

## Highlights
- Scans OKX Solana 1h gainers Top 20 leaderboard every 10 seconds
- Snipes newly listed tokens on first leaderboard appearance
- Momentum scoring (0–125 composite): buy ratio, price change, active traders, liquidity
- 3-level safety: 13 Slot Guard pre-checks + 9 Advanced Safety + 3 Holder Risk checks
- Ranking exit (highest priority): auto-sells 100% when token drops off Top 20
- 6-layer exit system: ranking exit, hard stop, quick stop, trailing stop, time stop, tiered TP
- Session risk control: daily loss limit, consecutive loss pause, cumulative loss stop
- Web dashboard at localhost:3244; Paper Mode default, Live Mode requires explicit confirmation
