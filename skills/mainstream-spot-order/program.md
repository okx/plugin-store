# Auto-Research Loop — SOL/USDC Spot Strategy

You are an auto-research agent. Your job is to iteratively improve `strategy.py` to maximize the backtest score.

## Loop

Repeat the following forever:

### 1. Observe
- Read `strategy.py` (current strategy)
- Read `results/latest.json` (last backtest results, if exists)
- Note the current score, sharpe, drawdown, num_trades

### 2. Hypothesize
Pick ONE focused change. Ideas ranked by expected impact:
- Tune a parameter (e.g., EMA period, RSI thresholds, ATR multiplier)
- Add/remove a signal from the ensemble
- Change entry/exit threshold
- Add a filter (e.g., volume, volatility regime)
- Modify position sizing logic (within 0.0-1.0 range)
- Add time-of-day or day-of-week filter
- Add mean-reversion signal for ranging markets
- Combine momentum + mean-reversion with regime detection

### 3. Implement
- Edit `strategy.py` with ONE change
- Keep the change small and testable

### 4. Test
```bash
python3 backtest.py --pair SOL
```

### 5. Evaluate
- Parse the JSON output for `score`
- Compare to previous score

### 6. Decide
- **Score improved**: Keep the change. Copy old strategy to `strategy_archive/strategy_v{N}.py`. Commit with message describing the change and score delta.
- **Score worse or same**: Revert `strategy.py` to previous version immediately. Do NOT keep bad changes.
- **Error**: Fix the error, re-test. If unfixable, revert.

### 7. Log
Print a one-line summary:
```
[iteration N] change="description" score=X.XX delta=+/-Y.YY result=KEPT/REVERTED
```

## Constraints
- ONLY modify `strategy.py`
- Never modify config.py, prepare.py, backtest.py, okx.py, collect.py, live.py
- No external pip packages — stdlib only
- target_position must stay in [0.0, 1.0] (spot only, no shorts)
- Keep strategy.py readable and well-commented
- Archive every improvement before making the next change

## Anti-Patterns to Avoid
- Overfitting to specific price patterns in the data
- Adding too many signals (>10) — complexity kills robustness
- Extremely tight parameters that only work on this dataset
- Removing all risk management (trailing stop, exit threshold)
