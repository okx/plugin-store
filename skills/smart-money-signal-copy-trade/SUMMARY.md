# smart-money-signal-copy-trade
Automated copy-trade bot that tracks OKX Smart Money, KOL, and Whale wallet signals on Solana with 15-check safety filters, cost-aware take profit, and a 7-layer exit system.

## Highlights
- Polls OKX Smart Money / KOL / Whale signals every 20 seconds
- Co-rider consensus: triggers only when 3+ smart wallets buy the same token simultaneously
- 15-check deep safety filters: market cap, liquidity, holders, dev rug, bundler, LP burn, K1 pump
- Cost-aware take profit (TP1 +5% / TP2 +15% / TP3 +30%) including fees and slippage in breakeven calc
- 7-layer exit system: liquidity emergency, hard stop, time-decay SL, tiered TP, trailing stop, trend stop
- Session risk control: consecutive loss pause and cumulative loss stop
- Hot-reload config — modify `config.py` without restarting the bot
- Web dashboard at localhost:3248; Paper Mode default, Live Mode requires explicit confirmation
