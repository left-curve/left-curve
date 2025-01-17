# Dango DEX

Dango DEX is a **fully onchain**, **MEV-resistent** spot order book exchange with **LVR-resistent passive liquidity provision**, and one of the two flagship apps on our upcoming DeFi focused L1 blockchain, [Dango](https://x.com/dango_zone).

Let's break down these jargons, starting from some backgrounds and historical recaps.

## Order book and market makers

In traditional finance (TradFi), trades happen primarily on **limit order books** (LOBs). To make a trade, a trader would place **limit orders** to the exchange, specifying the trade's direction (buy or sell), quantity, and a limit price. E.g., selling 1 share of Apple stock at $225 or better. The ledger that keeps track of all the active limit orders is the **order book**.

Most traders buy or sell a stock because they believe the price will go up or down; they are called **directional traders**.

**Market makers** is a type of traders that, instead of making directional bets, seek to profit by facilitating trades between directional traders. Here is how this works:

A market maker first computes a "fair price" of the stock, based on factors such as the stock's prevailing price in various exchanges, volatility, inventory, and time horizon. Then, it places BUY orders (also known as **bids**) at prices somewhat below the fair price, and SELL orders (a.k.a. **asks**) somewhat above it. The difference in the bid and ask prices is called the **spread**.

E.g., a market maker believes the fair price of AAPL to be $225. He then places a BUY order at $224.5 and a SELL order at $225.5. The spread in this case, is $1.

Market makers bet the stock's price will _go side ways_; in other words, there are roughly equal amount of BUY and SELL volume. If a trader comes to sell 1 share of AAPL to the market maker at $224.5, then another trader buys the same share at $225.5, the market maker makes $1 – the spread.

## Automated market makers

The world of DeFi, however, until very recently, have been dominated by another type of exchanges, the **automated market maker** (AMM).

Instead of maintaining a book of active orders and matching them, an AMM maintains an inventory of the two assets, and executes trades following a predefined **invariant** function. The simplest form of such invariants, [proposed by Martin Köppelmann](https://ethresear.ch/t/improving-front-running-resistance-of-x-y-k-market-makers/1281) and popularized by Uniswap, is:

$$
x \times y = K
$$

where $x$ and $y$ are the inventory quantities of the two assets, and $K$ a constant. Suppose a trader wishes to sell $x_{\mathrm{in}}$ amount of one asset; the AMM chooses an output amount $y_{\mathrm{out}}$ such that $K$ value is not changed, by solving the following equation:

$$
x y = (x + x_{\mathrm{in}}) (y - y_{\mathrm{out}})
$$

$$
y_{\mathrm{out}} = y - \frac{x y}{x + x_{\mathrm{in}}}
$$

AMM remained the predominant trading venue for much of DeFi's history because maintaining an order book and performing order matching is computationally expensive, exceeding what traditional blockchains can handle. It's only until recently when we start to see onchain order book implementations on high performance blockchains such as Solana, Sui, Dydx, and Hyperliquid.

Now, a question we must address is, why don't these newer generation chains keep using AMMs? Why don't TradFi markets also adopt AMM models? The answer is that AMMs, regardless of what invariant they use, have a fatal drawback known as [**loss-versus-rebalancing**](https://anthonyleezhang.github.io/pdfs/lvr.pdf) (LVR).

## LVR

Consider the following scenario: AAPL is trading at $225 at the Chicago Stock Exchange. Then, Apple releases a better-than-expected earnings report. Also assume Apple released it in New York, so NYSE traders receive the news first, pumping the price to $300. The news takes a few milliseconds to be delivered to Chicago, so it's still $225 there.

Suppose you're a market maker, sitting in your NYC office, managing your orders in the CSE. You have a BUY order at $224.5 and a SELL order at $225.5, as in the example from the previous section. What should you do now?

Apparently, you should cancel the old orders, and place new ones below and above the new price of $300. So you click the calcel order button... but oops, error: someone has snatched your SELL order before you're able to cancel it, buying AAPL at the low price of $225.5.

You just made a terrible trade: you sold an AAPL share for $225.5, when you could have sold it for $300!

The person who snatched the order is known as a **high frequency trader** (HFT). These traders invest hundreds of millions of dollars in fast private internet infrastructure, just so that they can pick up the **stale orders** a few nanoseconds faster than the market makers, and resell them at a higher price in NYSE.

As technology develops, trading in TradFi markets have become an arms race of who can be faster. This is a huge risk for market makers – they need to be able to update their stale orders faster than anybody else, or the loss from arbitrages can add up over time.

Now let's take our attention back to AMMs. AMMs, on the other hand, since they operate strictly on fixed, predefined invariants, _never update their stale quotes_. _Prices on AMMs are always outdated_.

The DeFi equivalent of HFT are called **searchers**. They identify stale prices on AMMs, and bribe block builders in order to get their transactions into earlier spots in the block, so that they carry out arbitrage faster than anyone else.
