# OKX Plugin Store -- User Guide

Welcome! This guide is for anyone who wants to use plugins with their AI assistant.
No coding experience needed. If you can type a message to an AI chatbot, you can use plugins.

---

## Table of Contents

1. [What is Plugin Store?](#1-what-is-plugin-store)
2. [Why OKX Plugin Store?](#2-why-okx-plugin-store)
3. [Quick Start](#3-quick-start)
4. [Plugin Directory](#4-plugin-directory)
5. [Usage Examples](#5-usage-examples)
6. [Managing Your Plugins](#6-managing-your-plugins)
7. [Safety and Disclaimer](#7-safety-and-disclaimer)
8. [FAQ](#8-faq)

---

## 1. What is Plugin Store?

Think of Plugin Store as an **App Store for your AI assistant**.

Your AI assistant (like Claude Code, Cursor, or OpenClaw) is already smart. But just
like your phone gets new abilities when you install apps, your AI assistant gets new
abilities when you install plugins.

- **Plugin Store** = the App Store
- **A plugin** = an app
- **Installing a plugin** = downloading an app to your phone

Without plugins, your AI assistant can answer questions and help with general tasks.
With plugins, it can do things like swap tokens on a decentralized exchange, track
what experienced traders are doing, or help you plan a liquidity position.

### See it in action

Here is what it looks like to install and use a plugin. You just type in plain
English (or any language), and your AI assistant does the rest:

```
You:    I want to plan a token swap on Uniswap.

AI:     I can help with that! Let me install the Uniswap Swap Planner plugin first.

        Installing uniswap-swap-planner... Done!

        Now, which tokens would you like to swap? For example, you could swap
        ETH for USDC on Ethereum mainnet.

You:    I want to swap 0.5 ETH for USDC.

AI:     Here is your swap plan:

        - From: 0.5 ETH
        - To: ~975 USDC (estimated)
        - Network: Ethereum
        - Estimated fee: ~$2.40

        Here is a direct link to execute this swap on Uniswap:
        [Open in Uniswap App]

        You will confirm the transaction in your own wallet before anything happens.
```

That is it. You described what you wanted, and the AI handled the rest.

---

## 2. Why OKX Plugin Store?

### Every plugin goes through a 4-stage security review

Before any plugin appears in the store, it must pass four layers of checks:

1. **Automated code scan** -- A program scans the plugin for known dangerous
   patterns (like code that tries to steal your passwords or inject hidden
   commands). Think of it as an airport security scanner.

2. **AI-powered behavior analysis** -- An AI reviewer reads the entire plugin to
   make sure it actually does what it claims to do and nothing else. This catches
   things that simple scanners might miss.

3. **Toxic flow detection** -- The system checks whether multiple small actions
   could combine into something harmful. For example, a plugin that reads sensitive
   files AND sends data to the internet would be flagged, even if each action
   alone seems harmless.

4. **Human review** -- A real person reviews the plugin before it goes live.
   Automated tools are good, but human judgment catches things machines cannot.

### Three trust levels tell you who made the plugin

Not all plugins come from the same place. We label each one so you know its origin:

| Badge | What it means | Example |
|-------|---------------|---------|
| **Official** | Made and maintained by the OKX team | Plugin Store itself |
| **Verified Partner** | Made by the actual protocol team (Uniswap, Polymarket, etc.) | Uniswap AI, Polymarket Agent |
| **Community** | Made by independent developers in the community | Meme Trench Scanner |

All three levels go through the same 4-stage review. The badge tells you *who*
made it, not whether it is safe. That said, Official and Verified Partner plugins
come from teams with established reputations.

### Simple installation

Installing a plugin takes one command. No downloading files, no editing
configuration files, no restarting anything. Just one line, and you are ready to go.

---

## 3. Quick Start

Getting started takes about 60 seconds. Here are the four steps:

### Step 1: Open your AI assistant

Open one of these supported AI assistants:

- **Claude Code** -- Anthropic's command-line AI tool
- **Cursor** -- AI-powered code editor
- **OpenClaw** -- Open-source AI assistant

If you already have one of these open, you are ready for the next step.

### Step 2: Install a plugin

Type this command in your AI assistant's terminal or chat:

```
npx skills add okx/plugin-store --skill uniswap-swap-planner
```

Replace `uniswap-swap-planner` with the name of any plugin you want. (See the
[Plugin Directory](#4-plugin-directory) below for the full list.)

Want to install the Plugin Store manager itself, which helps you browse and manage
everything? Run:

```
npx skills add okx/plugin-store --skill plugin-store
```

### Step 3: Start talking

Just describe what you want to do, in your own words:

```
You:    Help me plan a swap of 100 USDC to ETH on Arbitrum.
```

You do not need to use any special commands or syntax. Talk to your AI assistant
the same way you would talk to a helpful friend.

### Step 4: The AI uses the plugin automatically

Your AI assistant recognizes what you need, activates the right plugin, and
walks you through it. If a transaction is involved, you always get a chance to
review and confirm before anything happens.

```
AI:     I have prepared a swap plan for you:

        - From: 100 USDC
        - To: ~0.051 ETH (estimated)
        - Network: Arbitrum
        - Estimated fee: ~$0.08

        Here is the link to complete this swap on Uniswap. You will need to
        approve it in your wallet.
```

---

## 4. Plugin Directory

### Find by what you want to do

| I want to... | Plugin | Risk Level |
|--------------|--------|------------|
| Browse and install other plugins | Plugin Store | Starter |
| Participate in an AI hackathon | OKX BuildX Hackathon Guide | Starter |
| Plan a token swap on Uniswap | Uniswap Swap Planner | Starter |
| Plan a liquidity position | Uniswap Liquidity Planner | Starter |
| Learn about Uniswap v4 hook security | Uniswap V4 Security Foundations | Starter |
| Build apps with blockchain libraries | Uniswap Viem Integration | Starter |
| Swap tokens with AI-powered tools | Uniswap AI | Standard |
| Integrate swaps into a project | Uniswap Swap Integration | Standard |
| Pay for things using any token | Uniswap Pay With Any Token | Standard |
| Configure token auction contracts | Uniswap CCA Configurator | Standard |
| Deploy auction smart contracts | Uniswap CCA Deployer | Standard |
| Trade on prediction markets | Polymarket Agent Skills | Standard |
| Auto-trade newly launched meme tokens | Meme Trench Scanner | Advanced |
| Snipe tokens from OKX leaderboard | Top Rank Tokens Sniper | Advanced |
| Copy trades from smart money wallets | Smart Money Signal Copy Trade | Advanced |

### Find by risk level

Every plugin has a risk level that tells you what it can do and what you
should be aware of:

#### Starter -- Safe to explore

These plugins only **read** information. They help you plan, learn, and browse.
They never touch your wallet or make transactions.

*Examples: browsing available plugins, planning a swap (without executing it),
reading security documentation.*

**What to expect:** You ask a question, the AI gives you information. Nothing
moves, nothing gets signed, nothing costs money.

**Plugins at this level:**
- Plugin Store
- OKX BuildX Hackathon Guide
- Uniswap Swap Planner
- Uniswap Liquidity Planner
- Uniswap V4 Security Foundations
- Uniswap Viem Integration

---

#### Standard -- Transactions with your approval

These plugins can **prepare transactions**, but they always ask for your
confirmation before anything happens. Think of it like a shopping cart: the
plugin adds items, but you decide whether to check out.

*Examples: swapping tokens through Uniswap, placing a trade on Polymarket,
paying for a service with crypto.*

**What to expect:** The AI will walk you through what is about to happen, show
you the details (amounts, fees, destination), and ask you to confirm. You can
always say no.

**Plugins at this level:**
- Uniswap AI
- Uniswap Swap Integration
- Uniswap Pay With Any Token
- Uniswap CCA Configurator
- Uniswap CCA Deployer
- Polymarket Agent Skills

---

#### Advanced -- Automated trading (understand the risks first)

These plugins run **automated trading strategies**. They can buy and sell tokens
on your behalf based on signals, rankings, or market patterns. This is
like putting your trading on autopilot.

*Examples: automatically buying tokens that smart money wallets are buying,
sniping newly listed tokens on a leaderboard, scanning for meme token
opportunities.*

**What to expect:** These plugins can execute real trades with real money.
They come with paper trading modes (practice mode with no real money) so you
can learn how they work before risking anything.

**IMPORTANT:** These are high-risk tools. Token prices can drop to zero in
seconds. You can lose your entire investment. Always start with paper mode.

**Plugins at this level:**
- Meme Trench Scanner
- Top Rank Tokens Sniper
- Smart Money Signal Copy Trade

---

## 5. Usage Examples

Here are three real-world scenarios, from simple to complex, showing how plugins
work in practice.

### Scenario 1: Browse DeFi opportunities (Starter -- Safe)

You just heard about DeFi and want to know what is available. No money involved,
just exploring.

```
You:    What DeFi plugins are available? I am new to this and just want to
        look around.

AI:     Welcome! Here is what is available in the Plugin Store right now:

        TRADING
        - Uniswap AI -- Swap tokens with AI guidance
        - Uniswap Swap Planner -- Plan swaps and get direct links
        - Uniswap Swap Integration -- For integrating swaps into projects

        DEFI
        - Uniswap Liquidity Planner -- Plan liquidity positions
        - Uniswap Pay With Any Token -- Pay for things using any token

        PREDICTION MARKETS
        - Polymarket Agent Skills -- Trade on prediction markets

        AUTOMATED TRADING
        - Meme Trench Scanner -- Auto-trade meme tokens
        - Top Rank Tokens Sniper -- Snipe trending tokens
        - Smart Money Signal Copy Trade -- Copy smart money moves

        The Starter-level plugins (Swap Planner, Liquidity Planner) are a great
        place to begin. They only show you information -- nothing gets bought
        or sold.

        Want me to install one so you can try it?

You:    Sure, let us try the Swap Planner.

AI:     Installing uniswap-swap-planner... Done!

        You can now ask me things like:
        - "What would it cost to swap 1 ETH to USDC?"
        - "Show me the best route for swapping 500 USDC to WBTC"
        - "Plan a swap on Arbitrum"

        What would you like to try?

You:    What would it cost to swap 1 ETH to USDC on Ethereum?

AI:     Here is your swap plan:

        - From: 1 ETH
        - To: ~1,950 USDC (estimated)
        - Network: Ethereum mainnet
        - Estimated gas fee: ~$3.20
        - Price impact: < 0.01%

        This is just a plan -- no money has been spent. If you want to execute
        this swap, I can generate a link that takes you to the Uniswap app
        where you would confirm it with your own wallet.
```

**What happened:** You browsed plugins, installed one, and got swap information.
At no point was any money at risk. This is what Starter-level plugins are like.

---

### Scenario 2: Swap tokens on Uniswap (Standard -- With confirmation)

You have some ETH and want to swap it for USDC. This involves a real transaction,
so the AI will ask you to confirm.

```
You:    I want to swap 0.5 ETH for USDC on Ethereum.

AI:     I will prepare that swap for you using Uniswap.

        Here are the details:

        SWAP SUMMARY
        - Selling: 0.5 ETH
        - Buying: ~975.00 USDC (estimated)
        - Exchange rate: 1 ETH = ~1,950 USDC
        - Network: Ethereum mainnet
        - Estimated gas fee: ~$2.80
        - Minimum received (with 0.5% slippage): ~970.12 USDC

        Ready to proceed?

        I have generated a link to the Uniswap app where you can review
        and confirm this swap:
        [Open in Uniswap App]

        IMPORTANT: You will need to confirm the transaction in your wallet
        (like MetaMask). The swap only happens after you approve it there.
        You can cancel at any time before confirming.

You:    Looks good, I will click the link.

AI:     Great! Once you open the link, here is what to expect:

        1. The Uniswap app will show you the swap details
        2. Click "Swap" in the Uniswap interface
        3. Your wallet will pop up asking you to confirm
        4. Review the gas fee and total cost
        5. Click "Confirm" in your wallet

        The swap is final once confirmed in your wallet. Let me know if you
        have any questions or need help with anything else!
```

**What happened:** The AI prepared the transaction details, showed you everything
up front, and gave you a link. You still had to confirm in your own wallet. The AI
never had access to your funds -- it just helped you plan and prepare.

---

### Scenario 3: Track and copy smart money (Advanced -- With risk warnings)

You want to automatically follow what experienced traders (called "smart money")
are doing. This is an advanced strategy that involves real trading.

```
You:    I want to try copy trading. Can I automatically follow what smart money
        wallets are doing?

AI:     I can help set that up using the Smart Money Signal Copy Trade plugin.

        BEFORE WE START, here are some important things to understand:

        RISK WARNING
        - This plugin executes REAL trades with REAL money
        - Cryptocurrency prices can drop to zero within seconds
        - "Smart money" signals are not guarantees -- even experienced
          traders make losing bets
        - You could lose your entire invested amount
        - Past performance does not predict future results

        I strongly recommend starting in PAPER MODE first. Paper mode
        simulates trades without using real money, so you can see how the
        strategy performs before risking anything.

        Would you like to:
        (a) Start in paper mode (recommended -- no real money)
        (b) Learn more about how it works first
        (c) Set up live trading (real money at risk)

You:    Let us start with paper mode.

AI:     Good choice! Let me install the plugin first.

        Installing smart-money-signal-copy-trade... Done!

        PAPER MODE SETUP

        The bot is now configured in paper mode (DRY_RUN = True). Here is
        what it will do:

        - Monitor smart money wallets for buy/sell signals
        - Simulate trades based on those signals
        - Track simulated profit and loss
        - Show you a dashboard at http://localhost:3248

        No real money is involved. The bot will log what it WOULD have
        done so you can evaluate the strategy.

        The bot also includes safety features:
        - Dev/Bundler detection (avoids tokens with suspicious creators)
        - Cost-aware take-profit levels
        - Session risk controls (limits total exposure)

        Want me to start the paper trading bot now?

You:    Yes, start it.

AI:     Starting Smart Money Signal Copy Trade in paper mode...

        The bot is running. You can:
        - View the dashboard at http://localhost:3248
        - Check positions: the dashboard shows all tracked signals
        - See trade history: every simulated trade is logged

        I will let you know when the first signal comes in. In the meantime,
        the bot is watching smart money wallets for activity.

        Remember: when you are ready to consider live trading, we can review
        your paper trading results together first. There is no rush.
```

**What happened:** The AI warned you about the risks before doing anything. It
recommended paper mode (practice mode). It explained what the bot does and how to
monitor it. At every step, you were in control of the decision.

---

## 6. Managing Your Plugins

Once you have plugins installed, here is how to manage them. You can either
type these commands directly or just ask your AI assistant in plain English.

### See what you have installed

```
plugin-store installed
```

Or just ask: *"What plugins do I have installed?"*

### Update all your plugins to the latest version

```
plugin-store update --all
```

Or ask: *"Update all my plugins."*

### Uninstall a plugin you no longer need

```
plugin-store uninstall uniswap-swap-planner
```

Or ask: *"Remove the Uniswap Swap Planner plugin."*

### Get details about a specific plugin

```
plugin-store info uniswap-ai
```

Or ask: *"Tell me more about the Uniswap AI plugin."*

### Search for plugins by keyword

```
plugin-store search swap
```

Or ask: *"Are there any plugins for swapping tokens?"*

### List all available plugins

```
plugin-store list
```

Or ask: *"Show me all available plugins."*

### Update the Plugin Store itself

```
plugin-store self-update
```

Or ask: *"Update the Plugin Store to the latest version."*

---

## 7. Safety and Disclaimer

### What we do to protect you

Every plugin in the OKX Plugin Store goes through our **4-stage security review**
before it is published:

| Stage | What happens | What it catches |
|-------|-------------|-----------------|
| 1. Automated code scan | A program checks for dangerous code patterns | Malware, password theft, hidden commands |
| 2. AI behavior analysis | An AI reads the entire plugin to verify it does what it claims | Deceptive plugins, hidden functionality |
| 3. Toxic flow detection | The system checks if harmless-looking actions combine into something harmful | Sophisticated attacks that hide in plain sight |
| 4. Human review | A real person reviews the submission | Anything the automated tools missed |

All plugins -- Official, Verified Partner, and Community -- go through this
same process.

### What you should know

While we work hard to keep the Plugin Store safe, there are important things
to understand:

- **This is not financial advice.** The Plugin Store and its plugins are tools.
  They do not tell you what to buy, sell, or invest in. Any trading decisions
  are yours alone.

- **Do your own research (DYOR).** Before using any plugin that involves real
  money, take time to understand what it does, what the risks are, and whether
  it fits your situation.

- **Cryptocurrency is volatile.** Token prices can go up or down dramatically
  in very short periods. You can lose some or all of your money.

- **Start with paper mode.** Advanced trading plugins offer a practice mode that
  uses simulated money. Always try this first.

- **On-chain transactions are final.** Once a blockchain transaction is confirmed,
  it cannot be undone. Double-check everything before confirming.

- **Past performance is not a guarantee.** Just because a strategy worked before
  does not mean it will work again.

### How to report issues

If you find a plugin that behaves unexpectedly, seems malicious, or has a
security problem:

- **Email:** [security@okx.com](mailto:security@okx.com)
- **GitHub:** Open an issue at [github.com/okx/plugin-store](https://github.com/okx/plugin-store)

For security vulnerabilities, please use email instead of a public GitHub issue.
This gives the team time to fix the problem before it becomes widely known.

---

## 8. FAQ

### Is it free?

**Yes.** The Plugin Store and all plugins in it are free to install and use.
Some plugins interact with blockchain networks where you may pay network fees
(called "gas fees") for transactions, but the plugins themselves cost nothing.

### Is it safe?

Every plugin goes through a **4-stage security review** (automated scan, AI
analysis, toxic flow detection, and human review) before it appears in the store.
We also label each plugin with a trust badge (Official, Verified Partner, or
Community) so you know where it comes from.

That said, no review process is perfect. For plugins that handle real money,
always start with paper mode and do your own research.

### What data does Plugin Store collect?

**None.** The Plugin Store itself does not collect any personal data, usage data,
or wallet information. It is a tool that downloads plugin files to your computer --
that is all.

Individual plugins may interact with blockchain networks (which are public by
nature), but the Plugin Store itself does not track you.

### Where are plugins installed?

Plugins are installed into your AI assistant's local configuration folder on your
computer. They are just text files (instructions for your AI) stored alongside
your AI assistant's other settings.

- They stay on your machine
- They are not uploaded anywhere
- You can remove them at any time with the uninstall command

### Do I need to know how to code?

**No.** You interact with plugins by talking to your AI assistant in plain
language. The AI handles all the technical details. You just describe what you
want to do.

### Which AI assistants are supported?

Currently, Plugin Store works with:

- **Claude Code** -- Anthropic's command-line AI
- **Cursor** -- AI-powered code editor
- **OpenClaw** -- Open-source AI assistant

### Can I use multiple plugins at the same time?

**Yes.** You can install as many plugins as you want. Your AI assistant will
automatically use the right one based on what you ask it to do. For example, if
you ask about swapping tokens, it will use the swap plugin. If you ask about
prediction markets, it will use the Polymarket plugin.

### What if a plugin stops working?

Try these steps:

1. **Update the plugin:** Ask your AI *"Update all my plugins"* or run
   `plugin-store update --all`
2. **Reinstall it:** Uninstall and install again
3. **Check for issues:** Visit the plugin's GitHub page for known problems
4. **Report it:** If nothing works, report the issue (see
   [How to report issues](#how-to-report-issues) above)

### What is "paper mode"?

Paper mode (also called "dry run" or "simulation mode") lets you test a trading
strategy without risking real money. The plugin simulates what would happen --
it tracks buys, sells, profits, and losses -- but no actual transactions occur.
This is the safest way to evaluate an Advanced-level plugin before committing
real funds.

### What are "gas fees"?

Gas fees are small payments you make to the blockchain network to process your
transaction. Think of them like postage for a letter -- you pay a small amount
to have your transaction delivered and recorded. Gas fees vary depending on
network activity. Plugins that only read information (Starter level) do not
incur gas fees.

---

*This guide is maintained by the OKX Plugin Store team.
For developer documentation, see [FOR-DEVELOPERS.md](FOR-DEVELOPERS.md).
For partner documentation, see [FOR-PARTNERS.md](FOR-PARTNERS.md).*
