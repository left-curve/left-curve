# What is Dango?

Dango is a **DeFi-native Layer 1 blockchain** built from the ground up for trading. Where most blockchains are general-purpose platforms that happen to host DeFi apps, Dango inverts this: the chain is purpose-built around a DEX, with every infrastructure decision made to serve traders.

Developed by **Left Curve Software**, it describes itself as _"the one app for everything DeFi"_ — combining spot trading, perpetual futures, vaults, and lending within a single interface and a single unified margin account.

---

## Problems Dango Solves

### 1. Capital Inefficiency

On today's platforms, collateral is siloed. A trader on Aave must deposit separately from their dYdX position, their Uniswap LP, and so on. Dango's **Unified Margin Account** lets a single pool of collateral back spot trades, perpetual positions, and lending simultaneously.

### 2. Execution Quality & MEV

AMMs suffer from slippage and impermanent loss by design. Orders are also vulnerable to MEV — bots that front-run transactions for profit at the user's expense. Dango's on-chain **Central Limit Order Book (CLOB)** with periodic batch auctions eliminates both problems.

### 3. Terrible UX

DeFi onboarding is notoriously difficult: manage private keys, pay gas in native tokens, bridge assets across chains, juggle multiple wallets. Dango introduces **Smart Accounts** — a keyless system where accounts are secured by passkeys (biometrics) instead of seed phrases. Gas is paid in USDC.

### 4. Developer Inflexibility

EVM and Cosmos SDK give developers limited control over gas mechanics, scheduling, and account logic. Dango's **Grug execution environment** gives developers programmable gas fees, on-chain cron jobs, and customizable account logic — without hard forks.

---

## Key Stats

| Metric | Value |
|---|---|
| X / Twitter followers | ~111,000 |
| Testnet unique users | 180,000+ |
| Testnet transactions | 1.75M+ |
| Seed funding raised | $3.6M |
| Alpha Mainnet launch | January 2026 |

---

## What Makes Dango Different

Most chains compete on speed (TPS). Dango competes on **product design** — specifically by building its own execution environment (Grug) co-designed with the application layer. This _"app-driven infra development"_ enables features impossible or prohibitively expensive on EVM chains:

- On-chain CLOB with sub-second batch settlement
- Protocol-native cron jobs for automatic funding rate calculation
- Smart account architecture enabling biometric signing
- Zero gas fees
- Unified cross-collateral margin for all trading products
