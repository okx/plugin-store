# Macro Intelligence

Macro Intelligence trading skill

## Prerequisites

- **onchainos CLI** >= 2.1.0 — [install](https://docs.onchainos.com)
- **Python** >= 3.9
- Agentic wallet logged in: `onchainos wallet login`

## Install

```bash
pip install -r requirements.txt
```

## Quick Start

```bash
# 1. Login to wallet
onchainos wallet login

# 2. Start the dashboard
python3 macro_news.py
# Open http://localhost:3252
```

## Configuration

Edit `config.py` to adjust parameters. Hot-reload supported (no restart needed).

**Safe defaults**: The skill starts in paper/dry-run mode by default. Switch to live trading only after reviewing config.

## Risk Warning

This skill is for educational and research purposes only. Trading involves risk. Review all parameters carefully before enabling live mode.

## License

MIT
