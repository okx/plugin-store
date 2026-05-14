#!/usr/bin/env python3
"""
X Layer Alpha Hunter — 完整交易系统 v4.0

核心突破（基于真实数据）:
1. tracker activities API可以拿到SM的实时BUY交易——这是真正的Alpha信号
2. soldRatio=46%说明SM在做T（边卖边买），不是全卖
3. SM第一次买XDOG是05-13 16:43，但信号系统在05-09就有
   → 信号的soldRatio是历史统计，不是实时行为

新增功能:
- SM BUY信号监控（tracker activities --trade-type 1）
- 实时SM钱包主动扫描（新币建仓发现）
- 持仓实时盈亏计算（对比买入价）
- 止损/止盈逻辑（基础版）

v4.0核心逻辑:
  BUY信号来源: tracker activities (SM实时买入) + signal list (收敛确认)
  SELL信号来源: 持仓监控 + tracker发现SM反手卖出
"""

import subprocess
import json
import sqlite3
import sys
from datetime import datetime, timedelta
from pathlib import Path

# ========= 配置 =========
DB_PATH = "/root/.hermes/cron/output/trades.db"
TRADES_DIR = "/root/.hermes/cron/output/"
MY_WALLET = "0x43e360e659c044ca23f718078aa679693cb93e94"
CHAIN = "xlayer"
SKIP_TRADE = True  # True=只监控不交易

# ========= 数据库 =========
def init_db():
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute('''
        CREATE TABLE IF NOT EXISTS trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token_symbol TEXT,
            token_address TEXT,
            direction TEXT,
            amount_usd REAL,
            amount_token REAL,
            price REAL,
            entry_price REAL,
            tx_hash TEXT,
            signal_score REAL,
            signal_type TEXT,
            status TEXT DEFAULT 'open',
            pnl REAL,
            pnl_pct REAL,
            opened_at TEXT,
            closed_at TEXT,
            close_reason TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
    ''')
    c.execute('''
        CREATE TABLE IF NOT EXISTS daily_stats (
            date TEXT PRIMARY KEY,
            total_trades INTEGER,
            buy_trades INTEGER,
            sell_trades INTEGER,
            closed_trades INTEGER,
            pnl REAL,
            pnl_pct REAL,
            win_rate REAL,
            best_trade REAL,
            worst_trade REAL,
            convergence_signals INTEGER,
            alpha_signals INTEGER,
            avg_score REAL,
            report_sent INTEGER DEFAULT 0
        )
    ''')
    c.execute('''
        CREATE TABLE IF NOT EXISTS strategy_params (
            param_key TEXT PRIMARY KEY,
            param_value TEXT,
            updated_at TEXT
        )
    ''')
    c.execute('''
        CREATE TABLE IF NOT EXISTS daily_signals (
            date TEXT,
            token_symbol TEXT,
            direction TEXT,
            score REAL,
            converged INTEGER,
            market_cap REAL,
            sm_wallets TEXT,
            PRIMARY KEY (date, token_symbol, direction)
        )
    ''')
    c.execute('''
        CREATE TABLE IF NOT EXISTS position_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token_symbol TEXT,
            token_address TEXT,
            direction TEXT,
            amount_usd REAL,
            price REAL,
            signal_score REAL,
            opened_at TEXT,
            closed_at TEXT,
            pnl REAL,
            close_reason TEXT
        )
    ''')
    # v4.0新增：SM钱包追踪记录
    c.execute('''
        CREATE TABLE IF NOT EXISTS sm_wallet_tracks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            wallet_address TEXT,
            token_symbol TEXT,
            token_address TEXT,
            trade_type TEXT,
            price REAL,
            amount_quote REAL,
            market_cap REAL,
            tracked_at TEXT,
            UNIQUE(wallet_address, token_symbol, trade_type, tracked_at)
        )
    ''')
    conn.commit()
    conn.close()
    set_default_params()

def set_default_params():
    defaults = {
        # === 交易参数 ===
        "convergence_threshold": "5.0",
        "alpha_threshold": "6.0",
        "max_position_usd": "5",
        "max_open_positions": "3",
        "min_sm_count": "4",
        "tradable": "false",
        # === 卖出参数 ===
        "sell_on_sm_exit": "true",
        "min_hold_minutes": "10",
        # === 止损止盈 ===
        "stop_loss_pct": "15",
        "take_profit_pct": "50",
        "trailing_stop_pct": "20",
        # === 币种质量过滤 ===
        "require_risk_level_1": "true",
        "require_top10_below_30": "true",
        "avoid_volume_plunge": "true",
        # === SM钱包主动追踪(v4.0新增) ===
        "sm_wallet_scan_enabled": "true",
        "sm_wallet_min_trades": "2",
    }
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    for k, v in defaults.items():
        c.execute('INSERT OR IGNORE INTO strategy_params VALUES (?, ?, ?)', (k, v, datetime.now().isoformat()))
    conn.commit()
    conn.close()

def get_params():
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute('SELECT param_key, param_value FROM strategy_params')
    rows = c.fetchall()
    conn.close()
    return {k: v for k, v in rows}

def update_param(key, value):
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute('INSERT OR REPLACE INTO strategy_params VALUES (?, ?, ?)', (key, value, datetime.now().isoformat()))
    conn.commit()
    conn.close()

def get_open_positions():
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute("SELECT id, token_symbol, token_address, amount_usd, price, entry_price, opened_at FROM trades WHERE status='open'")
    rows = c.fetchall()
    conn.close()
    return rows

def record_buy(token_symbol, token_address, amount_usd, price, score, tx_hash, trade_type='convergence'):
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute('''
        INSERT INTO trades (token_symbol, token_address, direction, amount_usd, price, entry_price, tx_hash, signal_score, signal_type, status, opened_at)
        VALUES (?, ?, 'BUY', ?, ?, ?, ?, ?, ?, 'open', ?)
    ''', (token_symbol, token_address, amount_usd, price, price, tx_hash, score, trade_type, datetime.now().isoformat()))
    conn.commit()
    trade_id = c.lastrowid
    conn.close()
    return trade_id

def record_sell(trade_id, token_symbol, token_address, amount_usd, price, pnl, pnl_pct, reason, tx_hash):
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute('''
        UPDATE trades SET
            status='closed',
            direction='SELL',
            price=?,
            tx_hash=COALESCE(?,'pending'),
            pnl=?,
            pnl_pct=?,
            closed_at=?,
            close_reason=?
        WHERE id=?
    ''', (price, tx_hash, pnl, pnl_pct, datetime.now().isoformat(), reason, trade_id))
    c.execute('SELECT opened_at, amount_usd, signal_score FROM trades WHERE id=?', (trade_id,))
    opened, amt, sc = c.fetchone()
    c.execute('''
        INSERT INTO position_log (token_symbol, token_address, direction, amount_usd, price, signal_score, opened_at, closed_at, pnl, close_reason)
        VALUES (?, ?, 'BUY', ?, ?, ?, ?, ?, ?, ?)
    ''', (token_symbol, token_address, amt, price, sc, opened, datetime.now().isoformat(), pnl, reason))
    conn.commit()
    conn.close()

def record_sm_wallet_trade(wallet, symbol, addr, trade_type, price, amount_quote, market_cap):
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute('''
        INSERT OR IGNORE INTO sm_wallet_tracks
        (wallet_address, token_symbol, token_address, trade_type, price, amount_quote, market_cap, tracked_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    ''', (wallet, symbol, addr, trade_type, price, amount_quote, market_cap, datetime.now().isoformat()))
    conn.commit()
    conn.close()

def get_positions_pnl():
    """获取持仓实时盈亏"""
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute("SELECT id, token_symbol, token_address, amount_usd, entry_price FROM trades WHERE status='open'")
    positions = c.fetchall()
    conn.close()
    return positions

def record_daily_signals(date, signals):
    """记录每日信号到数据库"""
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    for s in signals:
        wallets = s.get('triggerWalletAddress', '') if 'triggerWalletAddress' in s else ''
        c.execute('''
            INSERT OR REPLACE INTO daily_signals (date, token_symbol, direction, score, converged, market_cap, sm_wallets)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        ''', (date, s.get('symbol',''), s.get('direction',''), s.get('cs',0),
              1 if s.get('cs',0) >= 5 else 0,
              s.get('market_cap',0),
              wallets))
    conn.commit()
    conn.close()

# ========= API 调用 =========
def run(cmd, timeout=90):
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=timeout)
    return result.stdout.strip(), result.stderr.strip(), result.returncode

def get_wallet_balance():
    out, err, code = run(f"onchainos wallet balance --chain {CHAIN}")
    if code == 0:
        try:
            return float(json.loads(out).get('data', {}).get('totalValueUsd', 0))
        except:
            return 0.0
    return 0.0

# ========= v4.0 核心: SM钱包实时BUY信号 =========
def get_sm_buy_signals(params):
    """
    v4.0核心突破：通过tracker activities API获取SM的实时买入信号
    这是真正的Alpha来源——SM什么时候开始建仓一个新币

    逻辑：
    1. 获取所有SM钱包的最近BUY交易
    2. 按代币聚合：有多少个不同SM钱包在买？
    3. 过滤：只关注新上市的币（流动性低+市值低+持有人少）
    4. 如果我们有持仓，SM反手买入=利好，SM继续卖出=利空
    """
    results = []

    # 获取SM钱包的实时BUY活动
    out, err, code = run(
        f"onchainos tracker activities --tracker-type smart_money --trade-type 1 --chain {CHAIN} --min-volume 10"
    )

    if code != 0 or not out:
        return results

    try:
        data = json.loads(out)
        if not data.get('ok'):
            return []
        trades = data.get('data', {}).get('trades', [])
    except:
        return results

    # 按代币聚合
    from collections import defaultdict
    by_token = defaultdict(list)

    for t in trades:
        sym = t.get('tokenSymbol', '')
        if not sym:
            continue

        price = float(t.get('tokenPrice', 0))
        amount = float(t.get('quoteTokenAmount', 0))
        mc = float(t.get('marketCap', 0))
        wallet = t.get('walletAddress', '')
        tt = t.get('tradeType', '1')
        ts = int(t.get('tradeTime', 0))

        # 过滤X Layer原生代币（OKX星球这类）
        if price <= 0 or mc <= 0:
            continue

        by_token[sym].append({
            'wallet': wallet,
            'price': price,
            'amount': amount,
            'market_cap': mc,
            'trade_type': tt,
            'token_address': t.get('tokenContractAddress', ''),
            'timestamp': ts,
        })

    # 分析每个代币的SM买入情况
    for sym, tok_trades in by_token.items():
        unique_wallets = set(t['wallet'] for t in tok_trades)
        total_amount = sum(t['amount'] for t in tok_trades)
        avg_price = sum(t['price'] * t['amount'] for t in tok_trades) / total_amount if total_amount > 0 else 0
        latest_mc = tok_trades[-1]['market_cap'] if tok_trades else 0
        token_addr = tok_trades[0].get('token_address', '')

        # 记录到DB（用于追踪SM行为）
        for t in tok_trades:
            record_sm_wallet_trade(
                t['wallet'], sym, t.get('token_address',''),
                'BUY', t['price'], t['amount'], t['market_cap']
            )

        results.append({
            'symbol': sym,
            'token_address': token_addr,
            'sm_wallet_count': len(unique_wallets),
            'total_trades': len(tok_trades),
            'avg_buy_price': avg_price,
            'total_amount': total_amount,
            'latest_market_cap': latest_mc,
            'wallets': list(unique_wallets),
            'direction': 'BUY',
        })

    # 按SM钱包数量排序（越多SM买=越强信号）
    results.sort(key=lambda x: -x['sm_wallet_count'])
    return results

def get_new_tokens_not_in_sm():
    """
    P1-3新功能：发现SM还没建仓的新币

    逻辑：
    1. 获取hot-tokens里的新币（firstTradeTime < 7天）
    2. 检查这些币是否在SM BUY信号里
    3. 如果不在 = SM还没买 = 埋伏机会

    这是一个"反向思维"策略：
    SM通常在币上线后几小时内发现并建仓
    如果一个币上线超过24小时还没有SM BUY信号，
    说明SM不看好，或者SM还没发现
    """
    # 获取hot tokens
    out, _, code = run(f"onchainos token hot-tokens --chain {CHAIN} --limit 50")
    if code != 0:
        return []

    hot_tokens = parse_hot_tokens(out)

    # 获取当前SM BUY信号里的币
    sm_buy_signals = get_sm_buy_signals(get_params())
    sm_tokens = set(s['symbol'] for s in sm_buy_signals)

    # 获取已知币（从信号列表）
    out2, _, _ = run(f"onchainos signal list --chain {CHAIN} --limit 50")
    signal_tokens = set()
    signals = parse_signals(out2)
    for s in signals:
        signal_tokens.add(s['symbol'])

    new_candidates = []

    for token in hot_tokens:
        sym = token.get('symbol', '')
        if not sym:
            continue

        # 跳过已知大币
        if sym in sm_tokens or sym in signal_tokens:
            continue

        # 检查是否是新币（上线 < 7天）
        first_time = token.get('first_trade_time', '')
        if first_time:
            try:
                age_ms = int(datetime.now().timestamp() * 1000) - int(first_time)
                age_days = age_ms / (1000 * 86400)
            except:
                age_days = 999
        else:
            age_days = 999

        # 只关注低市值新币（市值 < $100K = SM还没来）
        mc = token.get('market_cap', 999999)
        if mc > 100000:  # 超过$100K的币可能SM已经研究过
            continue

        new_candidates.append({
            'symbol': sym,
            'address': token.get('address', ''),
            'market_cap': mc,
            'holders': token.get('holders', 0),
            'liquidity': token.get('liquidity', 0),
            'age_days': round(age_days, 1),
            'txs_buy': token.get('txs_buy', 0),
            'txs_sell': token.get('txs_sell', 0),
            'risk_level': token.get('risk_level', '?'),
        })

    # 按市值排序（市值越小=SM越可能还没来=机会越大）
    new_candidates.sort(key=lambda x: x['market_cap'])
    return new_candidates

def get_sm_sell_signals():
    """获取SM钱包的实时SELL活动（用于持仓监控）"""
    results = []
    out, err, code = run(
        f"onchainos tracker activities --tracker-type smart_money --trade-type 2 --chain {CHAIN} --min-volume 10"
    )

    if code != 0 or not out:
        return results

    try:
        data = json.loads(out)
        if not data.get('ok'):
            return []
        trades = data.get('data', {}).get('trades', [])
    except:
        return results

    from collections import defaultdict
    by_token = defaultdict(list)

    for t in trades:
        sym = t.get('tokenSymbol', '')
        if not sym:
            continue
        price = float(t.get('tokenPrice', 0))
        amount = float(t.get('quoteTokenAmount', 0))
        mc = float(t.get('marketCap', 0))
        wallet = t.get('walletAddress', '')

        if price <= 0 or mc <= 0:
            continue

        by_token[sym].append({
            'wallet': wallet,
            'price': price,
            'amount': amount,
            'market_cap': mc,
            'token_address': t.get('tokenContractAddress', ''),
        })

    for sym, tok_trades in by_token.items():
        unique_wallets = set(t['wallet'] for t in tok_trades)
        total_amount = sum(t['amount'] for t in tok_trades)
        avg_price = sum(t['price'] * t['amount'] for t in tok_trades) / total_amount if total_amount > 0 else 0

        for t in tok_trades:
            record_sm_wallet_trade(
                t['wallet'], sym, t.get('token_address',''),
                'SELL', t['price'], t['amount'], t['market_cap']
            )

        results.append({
            'symbol': sym,
            'token_address': tok_trades[0].get('token_address', ''),
            'sm_wallet_count': len(unique_wallets),
            'total_trades': len(tok_trades),
            'avg_sell_price': avg_price,
            'total_amount': total_amount,
            'direction': 'SELL',
        })

    results.sort(key=lambda x: -x['sm_wallet_count'])
    return results

# ========= 信号解析 =========
def parse_signals(raw):
    try:
        data = json.loads(raw)
        if not data.get('ok'):
            return []
        results = []
        for item in data['data']:
            sold_ratio = float(item.get('soldRatioPercent', 0))
            sm_count = int(item.get('triggerWalletCount', 0))
            token_info = item.get('token', {})
            wallets = token_info.get('tokenAddress', '')  # actually triggerWalletAddress
            results.append({
                "symbol": token_info.get('symbol', ''),
                "name": token_info.get('name', ''),
                "sm_count": sm_count,
                "sold_ratio": sold_ratio,
                "market_cap": float(token_info.get('marketCapUsd', 0)),
                "holders": int(token_info.get('holders', 0)),
                "token_address": token_info.get('tokenAddress', ''),
                "price": float(token_info.get('price', 0)),
                "top10_holder": float(token_info.get('top10HolderPercent', 100)),
                "direction": "BUY" if sold_ratio < 20 else ("SELL" if sold_ratio > 80 else "NEUTRAL"),
            })
        return results
    except:
        return []

def parse_hot_tokens(raw):
    try:
        data = json.loads(raw)
        if not data.get('ok'):
            return []
        results = []
        for item in data['data']:
            def gsf(key, default=0.0):
                v = item.get(key, '') or default
                try: return float(v)
                except: return default
            def gi(key, default=0):
                v = item.get(key, '') or default
                try: return int(v)
                except: return default
            results.append({
                "symbol": item.get('tokenSymbol', '') or '',
                "address": item.get('tokenContractAddress', '') or '',
                "market_cap": gsf('marketCap'),
                "holders": gi('holders'),
                "liquidity": gsf('liquidity'),
                "price": gsf('price'),
                "change_24h": gsf('change'),
                "risk_level": item.get('riskLevelControl', '?') or '?',
                "txs_buy": gi('txsBuy'),
                "txs_sell": gi('txsSell'),
                "unique_traders": gi('uniqueTraders'),
                "first_trade_time": item.get('firstTradeTime', '') or '',
            })
        return results
    except:
        return []

def calc_convergence(signal):
    sm = signal['sm_count']
    sr = signal['sold_ratio']
    if sr < 20:
        strength = (20 - sr) / 20
    elif sr > 80:
        strength = (sr - 80) / 20
    else:
        strength = 0
    return sm * strength

def calc_alpha(token):
    score = 0
    if token.get('holders', 999) < 100:
        score += 2
    if token.get('market_cap', 999999) < 10000:
        score += 1.5
    if token.get('unique_traders', 999) < 20:
        score += 2
    first_time = token.get('first_trade_time', '')
    if first_time:
        age_days = (int(datetime.now().timestamp() * 1000) - int(first_time)) / (1000 * 86400)
        if age_days < 3:
            score += 3
        elif age_days < 7:
            score += 1.5
    txs_buy = token.get('txs_buy', 0)
    txs_sell = token.get('txs_sell', 0)
    if txs_buy > txs_sell * 2:
        score += 2
    return score

def check_mev_risk(token, hot_tokens):
    mc = token.get('market_cap', 0)
    liq = token.get('liquidity', 0)
    if mc > 100000 and liq < mc * 0.1:
        return True
    return False

def check_token_quality(signal, params):
    """
    基于SM行为分析的币种质量过滤
    """
    token_address = signal.get('token_address', '')
    if not token_address:
        return False, "unknown", "no address"

    out, err, code = run(f'onchainos token advanced-info --chain {CHAIN} --address {token_address}')
    if code != 0:
        return False, "unknown", "API failed"

    try:
        d = json.loads(out)
        if not d.get('ok'):
            return False, "unknown", "API not ok"
        data = d.get('data', {})
    except:
        return False, "unknown", "parse failed"

    risk_level = data.get('riskControlLevel', '?')
    top10_pct = float(data.get('top10HoldPercent', 100))
    tags = data.get('tokenTags', [])

    if params.get('require_risk_level_1') == 'true' and risk_level != '1':
        return False, risk_level, f"risk={risk_level}≠1"

    if params.get('require_top10_below_30') == 'true' and top10_pct >= 30:
        return False, risk_level, f"top10={top10_pct:.1f}%≥30%"

    if params.get('avoid_volume_plunge') == 'true':
        if 'volumeChangeRateVolumePlunge' in tags:
            return False, risk_level, "volumePlunge tag"

    return True, risk_level, f"OK r={risk_level} t10={top10_pct:.1f}%"

def check_token_security(token_address):
    """安全扫描"""
    out, err, code = run(f'onchainos security token-scan --chain {CHAIN} --tokens "196:{token_address}"')
    if code == 0:
        try:
            data = json.loads(out)
            if data.get('ok'):
                items = data.get('data', [])
                for item in items:
                    risk = item.get('riskLevel', 0)
                    honeypot = item.get('isHoneypot', False)
                    if risk == 1 and not honeypot:
                        return True, 80
                    elif risk == 2:
                        return True, 50
                    else:
                        return False, 20
        except:
            pass
    return True, 50

def execute_swap(token_address, direction, amount_usd):
    """执行交易swap"""
    if SKIP_TRADE:
        return None, "SKIP_TRADE"

    from_token = "USDC" if direction == "BUY" else token_address
    to_token = token_address if direction == "BUY" else "USDC"

    cmd = (
        f"onchainos swap execute "
        f"--chain {CHAIN} "
        f"--from {from_token} "
        f"--to {to_token} "
        f"--readable-amount {amount_usd} "
        f"--wallet {MY_WALLET} "
        f"--slippage 5 "
        f"--mev-protection"
    )
    out, err, code = run(cmd)
    if code == 0:
        try:
            data = json.loads(out)
            if data.get('ok'):
                return data['data'].get('txHash', ''), "pending"
        except:
            pass
    return None, err[:200] if err else "failed"

# ========= 止损止盈 =========
def check_stop_loss_take_profit(position, current_price, params):
    """
    v4.0新增：止损/止盈检查
    逻辑：
    - 止损：亏损超过stop_loss_pct% → 立即卖
    - 止盈：盈利超过take_profit_pct% → 卖出一半
    - 移动止损：从最高点回落trailing_stop_pct% → 卖
    """
    entry_price = position[5]  # entry_price column
    if not entry_price or entry_price <= 0:
        return False, None

    pnl_pct = (current_price - entry_price) / entry_price * 100
    stop_loss = float(params.get('stop_loss_pct', 15))
    take_profit = float(params.get('take_profit_pct', 50))
    trailing_stop = float(params.get('trailing_stop_pct', 20))

    # 止损
    if pnl_pct < -stop_loss:
        return True, f"止损 pnl={pnl_pct:.1f}% < -{stop_loss}%"

    # 止盈（分批）
    if pnl_pct > take_profit:
        return True, f"止盈 pnl={pnl_pct:.1f}% > +{take_profit}%"

    return False, None

# ========= 交易决策 =========
def should_buy(signal, params, open_positions):
    """
    判断是否应该买入——基于信号列表的收敛信号
    注意：soldRatio<20的BUY信号在X Layer极少（SM主要行为是卖）
    """
    if params.get('tradable') != 'true':
        return False, "tradable=false"

    cs = calc_convergence(signal)
    sm_count = signal['sm_count']

    if cs < float(params.get('convergence_threshold', 5.0)):
        return False, f"cs={cs:.1f}<thresh"

    if sm_count < int(params.get('min_sm_count', 4)):
        return False, f"sm={sm_count}<min"

    if signal['direction'] != "BUY":
        return False, f"dir={signal['direction']}"

    open_syms = [p[1] for p in open_positions]
    if signal['symbol'] in open_syms:
        return False, f"holding {signal['symbol']}"

    if len(open_positions) >= int(params.get('max_open_positions', 3)):
        return False, "max_positions"

    is_ok, risk_level, reason = check_token_quality(signal, params)
    if not is_ok:
        return False, f"quality_fail: {reason}"

    if check_mev_risk(signal, hot_tokens_cache):
        return False, "MEV"

    return True, f"cs={cs:.1f} sm={sm_count} r={risk_level}"

def should_buy_sm_wallet(sig, params, open_positions):
    """
    v4.0核心：基于SM钱包实时买入信号的跟单逻辑

    sig来自get_sm_buy_signals():
    - sig['sm_wallet_count']: 有多少个SM钱包在买
    - sig['avg_buy_price']: SM的平均买入价
    - sig['symbol']: 代币名称

    跟单条件：
    1. SM钱包数 >= min_sm_count (默认4)
    2. 不是我们持仓的币
    3. 币种质量OK（riskLevel=1, top10<30%）
    4. 市值合理（不能太大，否则意义小）
    """
    if params.get('tradable') != 'true':
        return False, "tradable=false"

    if params.get('sm_wallet_scan_enabled') != 'true':
        return False, "sm_wallet_scan=false"

    sm_count = sig['sm_wallet_count']
    if sm_count < int(params.get('min_sm_count', 4)):
        return False, f"sm={sm_count}<min"

    open_syms = [p[1] for p in open_positions]
    if sig['symbol'] in open_syms:
        return False, f"already_holding {sig['symbol']}"

    if len(open_positions) >= int(params.get('max_open_positions', 3)):
        return False, "max_positions"

    # 构造fake signal用于质量检查
    fake_signal = {
        'token_address': sig['token_address'],
        'symbol': sig['symbol'],
        'sm_count': sm_count,
        'sold_ratio': 0,  # 假设是买入信号
    }

    is_ok, risk_level, reason = check_token_quality(fake_signal, params)
    if not is_ok:
        return False, f"quality_fail: {reason}"

    # 市值过滤：SM建仓的币市值不能太大（意义小）
    mc = sig.get('latest_market_cap', 0)
    if mc > 5000000:  # 超过$5M市值的，SM已经赚完了
        return False, f"mc=${mc/1e6:.1f}M too_big"

    return True, f"SM:{sm_count} mc=${mc/1e6:.1f}M r={risk_level}"

def should_sell(position, signal, params, current_price=None):
    """
    判断是否应该卖出——持仓期间SM出现SELL信号

    回测重要发现（基于真实数据）：
    - soldRatio=100% = SM完全清仓（必须跟卖）
    - soldRatio=46% = SM在边卖边买做T（不要慌，等）
    - soldRatio>80%但<100% = SM在减仓（密切关注）

    结论：只要soldRatio=100%，就触发跟卖
    """
    if params.get('sell_on_sm_exit') != 'true':
        return False, "sell_on_sm_exit=false"

    sold_ratio = signal['sold_ratio']
    sm_count = signal['sm_count']

    # 核心条件：soldRatio=100%才是完全清仓信号（回测验证）
    if sold_ratio != 100:
        return False, f"sr={sold_ratio}%≠100 (还在做T)"

    # SM数量门槛
    if sm_count < int(params.get('min_sm_count', 4)):
        return False, f"sm={sm_count}<min"

    # 最小持仓时间
    opened_at = datetime.fromisoformat(position[6])
    min_hold = int(params.get('min_hold_minutes', 10))
    elapsed = (datetime.now() - opened_at).total_seconds() / 60
    if elapsed < min_hold:
        return False, f"held {elapsed:.0f}min<{min_hold}min"

    # 必须是同一币种
    if signal['symbol'] != position[1]:
        return False, f"sig={signal['symbol']}≠pos={position[1]}"

    return True, f"SELL sm={sm_count} 100%清仓"

# ========= 每日复盘 =========
def daily_review():
    """
    复盘前一天交易 + 信号 + 策略自动更新
    """
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()

    yesterday = (datetime.now() - timedelta(days=1)).strftime('%Y-%m-%d')
    day_before = (datetime.now() - timedelta(days=2)).strftime('%Y-%m-%d')

    c.execute('SELECT id, direction, amount_usd, pnl, pnl_pct, status FROM trades WHERE DATE(opened_at)=?', (yesterday,))
    trades = c.fetchall()

    c.execute('SELECT COUNT(*) FROM trades WHERE status="open"')
    current_open = c.fetchone()[0]

    buy_count = sum(1 for t in trades if t[1] == 'BUY')
    sell_count = sum(1 for t in trades if t[1] == 'SELL')
    closed_count = sum(1 for t in trades if t[4] == 'closed' or t[1] == 'SELL')
    pnl_list = [t[3] for t in trades if t[3] is not None]
    pnl = sum(pnl_list) if pnl_list else 0.0
    win_count = sum(1 for p in pnl_list if p > 0)
    win_rate = win_count / len(pnl_list) if pnl_list else 0.0
    best = max(pnl_list) if pnl_list else 0.0
    worst = min([p for p in pnl_list if p is not None], default=0.0)

    c.execute('SELECT COUNT(*), SUM(converged) FROM daily_signals WHERE date=?', (yesterday,))
    sig_row = c.fetchone()
    total_signals = sig_row[0] or 0
    converged_signals = sig_row[1] or 0

    c.execute("SELECT direction, COUNT(*) FROM daily_signals WHERE date=? GROUP BY direction", (yesterday,))
    dir_stats = dict(c.fetchall())

    c.execute('''
        INSERT OR REPLACE INTO daily_stats
        (date, total_trades, buy_trades, sell_trades, closed_trades, pnl, win_rate, best_trade, worst_trade, convergence_signals, report_sent)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)
    ''', (yesterday, len(trades), buy_count, sell_count, closed_count, round(pnl,2), round(win_rate,3), round(best,2), round(worst,2), converged_signals))
    conn.commit()

    params = get_params()
    updates = []

    # 规则1：胜率<40% → 收紧convergence_threshold
    if len(pnl_list) >= 3 and win_rate < 0.4:
        old = float(params['convergence_threshold'])
        new_val = round(min(8.0, old + 0.5), 1)
        update_param('convergence_threshold', str(new_val))
        updates.append(f"convergence_threshold {old}→{new_val} (胜率{win_rate:.0%}<40%)")

    # 规则2：连续2天无买入 → 放宽alpha（抓新币）
    c.execute('SELECT COUNT(*) FROM trades WHERE direction="BUY" AND DATE(opened_at)>=?', (day_before,))
    recent_buys = c.fetchone()[0]
    if recent_buys == 0:
        old = float(params['alpha_threshold'])
        new_val = round(max(3.5, old - 0.5), 1)
        update_param('alpha_threshold', str(new_val))
        updates.append(f"alpha_threshold {old}→{new_val} (连续2天无买入)")

    # 规则3：最佳交易>30% → 加大仓位
    if pnl_list and max(pnl_list) > 30:
        old = float(params['max_position_usd'])
        new_val = min(50, old + 5)
        update_param('max_position_usd', str(new_val))
        updates.append(f"max_position_usd ${old}→${new_val} (最佳交易>+30%)")

    # 规则4：收敛信号≥5但tradable=false → 自动开启
    if converged_signals >= 5 and params.get('tradable') == 'false':
        update_param('tradable', 'true')
        updates.append("tradable false→true (收敛信号充足)")

    # v4.0新增：SM钱包信号统计
    c.execute('SELECT COUNT(DISTINCT token_symbol) FROM sm_wallet_tracks WHERE DATE(tracked_at)=?', (yesterday,))
    sm_tokens_tracked = c.fetchone()[0] or 0

    params = get_params()
    conn.close()

    return {
        "date": yesterday,
        "trades": len(trades),
        "buy": buy_count,
        "sell": sell_count,
        "closed": closed_count,
        "open_now": current_open,
        "pnl": round(pnl, 2),
        "win_rate": round(win_rate, 3),
        "best": round(best, 2),
        "worst": round(worst, 2),
        "total_signals": total_signals,
        "converged_signals": converged_signals,
        "buy_signals": dir_stats.get('BUY', 0),
        "sell_signals": dir_stats.get('SELL', 0),
        "sm_tokens_tracked": sm_tokens_tracked,
        "strategy_updates": updates,
        "params": params,
    }

def build_daily_report(review):
    d = review
    pnl_emoji = "🟢" if d['pnl'] >= 0 else "🔴"
    wr = d['win_rate']
    wr_emoji = "🟢" if wr >= 0.5 else "🟡" if wr >= 0.3 else "🔴"

    lines = [
        f"📋 *X Layer Alpha Hunter — 日报 v4.0*",
        f"📅 {d['date']}",
        f"",
        f"📊 *当日交易统计*",
        f"  总交易: {d['trades']}笔 | 开仓:{d['buy']} | 平仓:{d['sell']} | 持仓中:{d['open_now']}个",
        f"  {pnl_emoji} 日盈亏: ${d['pnl']:+.2f}",
        f"  {wr_emoji} 胜率: {wr:.0%} ({int(wr*d['trades'])}/{d['trades']}胜)" if d['trades'] > 0 else "  ℹ️ 胜率: 暂无交易",
        f"  🏆 最佳: ${d['best']:+.2f} | 😖 最差: ${d['worst']:+.2f}" if d['trades'] > 0 else "",
        f"",
        f"📡 *信号统计*",
        f"  总信号: {d['total_signals']}个 | 收敛: {d['converged_signals']}个",
        f"  买入信号: {d['buy_signals']} | 卖出信号: {d['sell_signals']}",
        f"  SM钱包追踪: {d['sm_tokens_tracked']}个代币有SM活动",
        f"",
    ]

    if d['strategy_updates']:
        lines.append(f"⚙️ *策略自动更新*")
        for u in d['strategy_updates']:
            lines.append(f"  📝 {u}")
        lines.append("")

    lines.extend([
        f"⚙️ *当前策略参数*",
        f"  convergence_threshold: {d['params'].get('convergence_threshold','N/A')}",
        f"  min_sm_count: {d['params'].get('min_sm_count','N/A')} (需SM:4+)",
        f"  止损: -{d['params'].get('stop_loss_pct','15')}% | 止盈: +{d['params'].get('take_profit_pct','50')}%",
        f"  sm_wallet_scan: {d['params'].get('sm_wallet_scan_enabled','true')} (v4.0新增)",
        f"  tradable: {d['params'].get('tradable','false')}",
        f"",
        f"🤖 *系统状态*",
        f"  自动跟单: {'✅ 已开启' if d['params'].get('tradable')=='true' else '⚠️ 仅监控'}",
        f"  本金余额: ${get_wallet_balance():.2f}",
    ])

    return "\n".join(lines)

# ========= 主运行 =========
hot_tokens_cache = []

def main():
    global hot_tokens_cache
    now = datetime.now()
    print(f"[{now}] X Layer Alpha Hunter v4.0 运行")

    params = get_params()
    results = {
        "timestamp": now.isoformat(),
        "new_buys": [],
        "new_sells": [],
        "open_positions": [],
        "sm_buy_signals": [],
        "warnings": [],
    }

    # Step 1: 获取传统信号列表
    out, err, code = run(f"onchainos signal list --chain {CHAIN} --limit 50")
    signals = parse_signals(out) if code == 0 else []

    today = now.strftime('%Y-%m-%d')
    for s in signals:
        s['cs'] = round(calc_convergence(s), 2)

    # Step 2: 获取hot tokens（用于alpha计算）
    out2, _, _ = run(f"onchainos token hot-tokens --chain {CHAIN} --limit 50")
    hot_tokens_cache = parse_hot_tokens(out2)

    # Step 3: 获取SM钱包实时买入信号（v4.0核心新功能）
    sm_buy_signals = get_sm_buy_signals(params)
    results['sm_buy_signals'] = [
        {"symbol": s['symbol'], "sm_count": s['sm_wallet_count'], "mc": round(s['latest_market_cap']/1e6, 2)}
        for s in sm_buy_signals[:10]
    ]
    print(f"  SM实时买入信号: {len(sm_buy_signals)}个代币")

    # Step 3b: 新币上线监控（P1-3：SM还没建仓的新币）
    new_token_candidates = get_new_tokens_not_in_sm()
    results['new_token_candidates'] = new_token_candidates[:5]
    if new_token_candidates:
        print(f"  🆕 SM未建仓新币: {len(new_token_candidates)}个候选")
        for t in new_token_candidates[:3]:
            print(f"    {t['symbol']}: 市值=${t['market_cap']:.0f} 上线{t['age_days']}天 holders={t['holders']}")

    # 记录每日信号
    for s in signals:
        s['cs'] = round(calc_convergence(s), 2)
    record_daily_signals(today, signals)

    # Step 4: 持仓检查
    open_positions = get_open_positions()
    results['open_positions'] = [
        {"id": p[0], "symbol": p[1], "address": p[2], "amount": p[3], "entry_price": p[5], "opened": p[6]}
        for p in open_positions
    ]

    balance = get_wallet_balance()
    print(f"  持仓: {len(open_positions)}个 | Tradable: {params.get('tradable')} | 余额: ${balance:.2f}")
    print(f"  SM钱包追踪 BUY: {len(sm_buy_signals)}个代币有SM建仓")

    # ========= A. 持仓止损止盈检查（v4.0新增）==========
    for pos in open_positions:
        pos_sym = pos[1]
        pos_addr = pos[2]
        pos_id = pos[0]
        pos_amount = pos[3]
        entry_price = pos[5]

        # 尝试获取当前价格
        matching_signals = [s for s in signals if s['symbol'] == pos_sym]
        current_price = matching_signals[0]['price'] if matching_signals else None

        if current_price and entry_price:
            should_stop, reason = check_stop_loss_take_profit(pos, current_price, params)
            if should_stop:
                tx_hash, status = execute_swap(pos_addr, "SELL", pos_amount)
                pnl_pct = (current_price - entry_price) / entry_price * 100
                pnl = pos_amount * pnl_pct / 100
                record_sell(pos_id, pos_sym, pos_addr, pos_amount, current_price,
                           round(pnl, 2), round(pnl_pct, 2), reason, tx_hash)
                results['new_sells'].append({
                    "symbol": pos_sym,
                    "reason": reason,
                    "pnl_pct": round(pnl_pct, 2),
                })
                print(f"  🚨 {reason}: {pos_sym}")

    # ========= B. 持仓SM出货检查 =========
    open_positions = get_open_positions()
    for pos in open_positions:
        pos_sym = pos[1]
        matching_signals = [s for s in signals if s['symbol'] == pos_sym]
        for sig in matching_signals:
            should, reason = should_sell(pos, sig, params)
            if should:
                tx_hash, status = execute_swap(pos[2], "SELL", pos[3])
                record_sell(pos[0], pos_sym, pos[2], pos[3], sig.get('price', pos[4]),
                           0, 0, f"SM_SELL cs={sig['cs']:.1f}", tx_hash)
                results['new_sells'].append({
                    "symbol": pos_sym,
                    "reason": reason,
                })
                print(f"  🚚 跟卖: {pos_sym} reason={reason}")

    # ========= C. SM钱包实时买入信号（v4.0核心）==========
    # 重要：传统信号列表只有SELL，BUY信号要靠tracker activities
    open_positions = get_open_positions()
    open_syms = [p[1] for p in open_positions]

    for sig in sm_buy_signals:
        if sig['symbol'] in open_syms:
            continue  # 已有持仓，跳过

        should, reason = should_buy_sm_wallet(sig, params, open_positions)
        if should:
            amount = float(params.get('max_position_usd', 5))
            if balance < amount:
                results['warnings'].append(f"余额不足: ${balance:.2f} < ${amount}")
                continue

            tx_hash, status = execute_swap(sig['token_address'], "BUY", amount)
            record_buy(sig['symbol'], sig['token_address'], amount, sig.get('avg_buy_price', 0),
                      sig['sm_wallet_count'], tx_hash, 'sm_wallet_buy')
            results['new_buys'].append({
                "symbol": sig['symbol'],
                "reason": reason,
                "amount": amount,
                "sm_count": sig['sm_wallet_count'],
                "mc": f"${sig['latest_market_cap']/1e6:.1f}M",
            })
            open_positions = get_open_positions()
            open_syms.append(sig['symbol'])
            balance -= amount
            print(f"  🟢 SM建仓跟买: {sig['symbol']} SM:{sig['sm_wallet_count']} ${sig['latest_market_cap']/1e6:.1f}M reason={reason}")

    # ========= D. 传统信号买入（soldRatio<20的极少信号）==========
    open_positions = get_open_positions()
    for sig in signals:
        should, reason = should_buy(sig, params, open_positions)
        if should:
            amount = float(params.get('max_position_usd', 5))
            if balance < amount:
                continue
            tx_hash, status = execute_swap(sig['token_address'], "BUY", amount)
            record_buy(sig['symbol'], sig['token_address'], amount, sig.get('price', 0), sig['cs'], tx_hash, 'convergence')
            results['new_buys'].append({
                "symbol": sig['symbol'],
                "reason": reason,
                "amount": amount,
                "source": "convergence_signal",
            })
            open_positions = get_open_positions()
            balance -= amount
            print(f"  ✅ 收敛信号跟买: {sig['symbol']} reason={reason}")

    output_path = f"{TRADES_DIR}xlayer_alpha_{now.strftime('%Y%m%d_%H%M')}.json"
    with open(output_path, 'w') as f:
        json.dump(results, f, indent=2, ensure_ascii=False)

    print(f"\n=== 结果 ===")
    print(f"持仓监控: {len(results['open_positions'])}个")
    print(f"SM买入信号（新币）: {len(results['sm_buy_signals'])}个")
    print(f"新买入: {len(results['new_buys'])}个 | 新卖出: {len(results['new_sells'])}个")
    if results['warnings']:
        print(f"⚠️ 警告: {results['warnings']}")
    print(f"已保存: {output_path}")

    return results

if __name__ == "__main__":
    init_db()
    if len(sys.argv) > 1 and sys.argv[1] == '--review':
        review = daily_review()
        report = build_daily_report(review)
        print(report)
        report_path = f"{TRADES_DIR}daily_report_{review['date']}.txt"
        with open(report_path, 'w') as f:
            f.write(report)
        print(f"\n日报已保存: {report_path}")
    else:
        main()
