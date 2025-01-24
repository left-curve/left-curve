# Dango DEX

Dango DEX is an onchain limit order book exchange that tackles three main challenges faced by today's DEXs: the inaccessibility of order book market making to retail investors due to the high level of sophistication required; reduced yield for AMM LPs due to arbitrage flow; and malicious MEV. Dango DEX enshrines its order book with a passive liquidity pool that actively adjusts its quotes utilizing a low latency oracle, and clears orders using frequent, uniform price, sealed bid auctions. Overall, Dango DEX seeks to democratize market making on order books; minimize toxic arbitrage flow while maximize organic, non-arbitrage flow; and offer fair prices to retail traders.

Dango DEX is one of the two flagship apps of our upcoming DeFi ecosystem [Dango](https://x.com/dango_zone), besides the cross-collateralized **credit account**. To learn more about the credit account, check out this video. **[TODO: ADD URL]**

This paper is laid out as follows: 1) identifying the problems in today's DeFi exchanges; 2) analyzing the causes of the problems; and 3) proposing our solutions.

## Background

### TradFi

In traditional finance (TradFi) markets, **limit order book** (LOB) is the primary venue for trading. A limit order consists of the quantity of asset the trader wishes to buy/sell, and a limit price at which or better the trade must be executed.

**Market makers** (MMs) facilitate trades by strategically placing orders below and above the prevailing market price. E.g., suppose Apple stock (AAPL) is trading at $200. An MM may place a BUY order at $199.5 and a SELL order at $200.5. The $1 difference is known as the **spread**. If a trader sells AAPL to the MM by taking the said BUY order, then another trader buys AAPL by taking the said SELL order, the MM has made $1. This is typically described as the MM "making money on the spread".

MMs bet the stock's price goes side ways, i.e. there is roughly the same buy and sell volume. If the market only goes one way, the MM accumulates one side of the inventory, which is the underperforming side. E.g., if there is consistently more sellers of AAPL than buyers, the MM's BUY orders get consistently picked up more often than his SELL orders do, he would accumuate a large inventory of AAPL, an asset that's going down in price. He would underperform the trading strategy that simply holds the initial inventory without market making. This is known as the **inventory risk** which is rooted in the asset's price volatility. MMs usually use **hedging** to mitigate this risk.

Another major risk faced by MMs comes from **information asymmetry**. Suppose Apple releases a better-than-expected earnings report, causing the "fair value" of its stock to jump to $300. However, the MM is still placing orders around the **stale price** of $200, either because he isn't aware of the news or is slow to update the quotes. An **informed trader** or **high frequency trader** (HFT) is one who is well informed on the news and is able to execute trades faster. He is able to pick up the MM's SELL order at $200.5 and immediately resell the stock for $300, pocketing the arbitrage gain. The MM loses value because he has sold the stock at a much lower-than-market price. Both MMs and HFTs invest significant amount of resources to improve their execution speed in an ["arms race"](https://academic.oup.com/qje/article/130/4/1547/1916146). This is a very high level of sophistication that makes market making inaccessible to most retail investors.

### DeFi

Maintaining a LOB and executing orders is computationally costly. For this reason, onchain finance has historically relied on **constant function market makers** (CFMMs) instead of LOBs. (That is, until the recent advent of high performance blockchains such as Solana, Sui, Dydx, and Hyperliquid.)

A CFMM, instead of maintaining a book of limit orders, maintains a pool of liquidity and quotes prices based on a predefined **invariant** function. An invariant is a function $f(x, y)$ that yields the same value before and after a trade, where $x$ and $y$ are the quantities of the **base asset** and the **quote asset**, respectively (known as the pool's **reserves**):

$$
f(x, y) = K \ (\mathrm{constant})
$$

Suppose a pool contains base asset $\mathtt{A}$ and quote asset $\mathtt{B}$ of reserves $A$ and $B$, respectively, and a trader wishes to swap $a$ amount of $\mathtt{A}$ into $\mathtt{B}$. The pool would determine the output amount $b$ by solving the equation:

$$
f(A, B) = f(A + a, B - b) = K
$$

Similarly, a swap from $b$ amount of $\mathtt{B}$ into $\mathtt{A}$ will have its output amount $a$ determined by:

$$
f(A, B) = f(A - a, B + b) = K
$$

Note that this doesn't consider trading fees. Since there is no such thing as "spread" in CFMMs as in LOBs, the pool makes money for its **liquidity providers** (LPs) by charging a fee on each trade. Specifically, a small portion of the trade's output is deducted and injected into the pool. The value of $K$ slightly increases as a result. As such, $K$ can be considered as a measurement of how much liquidity there is in the pool, regardless of the asset prices.

CFMMs share the same types of risks as LOBs, but to different degrees. Firstly, LPs in an CFMM pool bet the two assets' relative price stays roughly constant. If one asset's price drops relative to the other, the pool accumulates this underperforming asset. Not considering fees, this would underperform the strategy of simply holding the two assets and not market making. This is known as **impermanent loss** (IL). IL is essentially the same thing as inventory risk for LOBs, caused by the asset's volatility, and can be mitigated through hedging.

In terms of information asymmetry, however,  CFMMs are categorically worse than LOBs. A traditional CFMM _never_ adjusts its quotes in response to new information. Therefore, from an LP's perspective, a CFMM always trades at worse-than-market prices. The loss incurred from this is known as [**loss-versus-rebalancing** (LVR)](https://arxiv.org/abs/2208.06046).

A **searcher** is a trader who scans onchain DEXs for stale prices, and executes CEX-DEX arbitrages. Since arbitraging is highly competitive, searchers share portions of their arbitrage gains with the chain's block builder by paying "tips".

There are other ways besides arbitrage with which searchers make money. Once of these is **sandwich attacks**, where a searcher bribes the block builder to insert transactions (txns) immediately before and after a user's trade. These txns manipulate prices in the DEX, give the user a worse execution price, while profiting the trader. Sandwich attack is not inheritly a problem of CFMMs; onchain LOBs can be similarly attacked. Instead, it's a result of the fact that user orders are broadcasted transparently through the network. In comparison, user orders in CEXs are kept private unless filled.

### The problems

Through the above discussions, we have identified three problems with TradFi and DeFi exchanges:

1. Market making in LOBs is not accessible to retail investors because of the high level of sophistication required, which is in larger part a result of the HFT arms race.
2. CFMM democratizes market making to retail investors, but they generally don't make money or even lose money because of LVR.
3. Traders are susceptible to sandwich attacks due to the lack of privacy.

We do not aim to solve IL / inventory risk, because it's rooted in the assets' volalitity, not a problem with DEX design. We can imagine vaults that automatically deploy hedging strategies to mitigate it.

## Our solution

We propose solving the above problems as follows:

- Create a LOB that has an enshrined passive liquidity pool. The pool will place orders in the LOB following a CFMM invariant.
- In order to mitigate LVR, we:
  - make available a high-frequency, low-latency oracle reporting the latest prevailing market prices;
  - incorporate this oracle feed into the liquidity pool's CFMM invariant;
  - give the pool priority in adjusting its quotes over other traders.
- In order to mitigate MEV, we:
  - use a private mempool so that user orders aren't public;
  - use frequent, uniform price, sealed-bid auctions to match and execute orders, so that HFTs don't have time advantage over other traders.

First, let's discuss how to incorporate a passive CFMM pool into a LOB. Let's start with the simplest form of CFMM invariants, the **xyk invariant**.

### Passive liquidity on a LOB following the xyk invariant

The xyk invariant, [proposed by Martin Köppelmann](https://ethresear.ch/t/improving-front-running-resistance-of-x-y-k-market-makers/1281), has the form:

$$
f(x, y) = x \cdot y = K
$$

How would the pool place orders in a LOB, following this invariant? Let's start with the BUY side. Suppose the pool has reserves $x = A$ and $y = B$. It places a BUY order, offering $b_{\mathrm{bid}}$ units of the quote asset $\mathtt{B}$ in exchange for $a_{\mathrm{bid}}$ units of the base asset $\mathtt{A}$, at price $p$. The invariant must hold:

$$
A B = (A + a_{\mathrm{bid}}) (B - b_{\mathrm{bid}})
$$

By definition:

$$
p = \frac{b_{\mathrm{bid}}}{a_{\mathrm{bid}}}
$$

Putting these together, we can easily solve:

$$
a_{\mathrm{bid}} = -A + \frac{B}{p}
$$

Similarly, on the SELL side, we have:

$$
A B = (A - a_{\mathrm{ask}}) (B + b_{\mathrm{ask}})
$$

$$
p = \frac{b_{\mathrm{ask}}}{a_{\mathrm{ask}}}
$$

$$
a_{\mathrm{ask}} = A - \frac{B}{p}
$$

It's immediately obvious that in both cases, if $p = \frac{B}{A}$, then $a = 0$. This is a special price, at which the pool does not offer to buy or sell in any amount, we denote as the **pool price** $p_{\mathrm{pool}}$. In the most general case, $p_{\mathrm{pool}}$ is _the price at which the trade swapping an infinitesimal amount of base asset into the quote asset is executed_:

$$
p_{\mathrm{pool}}(x, y) = - \frac{\mathrm{d}y}{\mathrm{d}x}
$$

Since $x$ and $y$ follows $f(x, y) = K$, using the chain rule of multivariant functions, we can get:

$$
p_{\mathrm{pool}}(x, y) = \frac{\frac{\partial f}{\partial x}}{\frac{\partial f}{\partial y}}
$$

For the xyk invariant specifically, this is:

$$
p_{\mathrm{pool}}(x, y) = \frac{y}{x}
$$

Intuitively, $p_{\mathrm{pool}}$ is the "inherit", "equilibrium" price of the pool implied by its reserves. The pool will place orders around this price.

At any price $p < p_{\mathrm{pool}}$, $a_{\mathrm{bid}} > 0$. This can be understood as the pool offers to buy $a_{\mathrm{bid}}$ amount of the base asset between prices $p$ and $p_{\mathrm{pool}}$.

Of course, prices LOBs are discret, separated by **ticks**. Let's say the tick size is $\Delta p$. Consider the price one tick above $p$, $p' = p + \Delta p$:

$$
a'_{\mathrm{bid}} = -A + \frac{B}{p'}
$$

$$
\Delta a_{\mathrm{bid}} = a_{\mathrm{bid}} - a'_{\mathrm{bid}} = B \left(\frac{1}{p} - \frac{1}{p'} \right) = B \frac{\Delta p}{p (p + \Delta p)}
$$

This is the quantity of BUY order that the pool will place at price $p$.

Similarly, one the SELL side, for two prices $p' = p - \Delta p$ and $p' > p_{\mathrm{pool}}$:

$$
\Delta a_{\mathrm{ask}} = a_{\mathrm{ask}} - a'_{\mathrm{ask}} = B \left(\frac{1}{p} - \frac{1}{p'} \right) = B \frac{\Delta p}{p (p - \Delta p)}
$$

#### Example

Consider a pool containing base asset SOL of quantity $A = 1$ and quote asset USD of quantity $B = 200$. Pool price $p_{\mathrm{pool}} = \$ 200$. Plotting the order sizes $\Delta a_{\mathrm{bid}}$, $\Delta a_{\mathrm{ask}}$ as well as cumulative buy/sell demand agianst price $p$:

![Order size and depth following the xyk invariant](1-xyk.png)

#### Brief conclusion

We have established the concept of pool price $p_{\mathrm{pool}}$, and a way to derive BUY/SELL order size $\Delta a_{\mathrm{bid}}$ and $\Delta a_{\mathrm{ask}}$ in relation to a given price $p$.

Looking at the order depth chart of the xyk invariant, it puts liquidity roughly evenly over a wide range of prices. This in capital inefficient; ideally, we want to concentrate the liquidity in the region close to $p_{\mathrm{pool}}$.

Additionally, this invariant is susceptible to LVR, as $p_{\mathrm{pool}}$ does not adjust in response to the change of price in CEXs.

### Passive liquidity on a LOB following the Solidly invariant

The Solidly invariant, [conceived by Andre Cronje](https://x.com/0xdef1/status/1482133989720743946) and popularized by Velodrome and its forks, has the form:

$$
f(x, y) = x^3 y + x y^3 = K
$$

This formula assumes the two assets have the same price. In case they're not the same price, and we have an oracle feed indicating that one unit of $x$ is equivalent in value to $R$ units of $y$, the formula can be updated to:

$$
f(x, y) = x^3 \left( \frac{y}{R} \right) + x \left( \frac{y}{R} \right)^3
$$

The pool price is:

$$
p_{\mathrm{pool}}(x, y) = \frac{3 R^2 x^2 y + y^3}{R^2 x^3 + 3 R x y^2}
$$

Let's derive $a_{\mathrm{bid}}$ the same way as we did for the xyk pool. Suppose the pool as reserves $A$ and $B$ prior to the swap:

$$
f(A, B) = A^3 \left( \frac{B}{R} \right) + A \left( \frac{B}{R} \right)^3 = K
$$

On the BUY side, a trader inputs $a_{\mathrm{bid}}$ units of base asset, receives $b_{\mathrm{bid}}$ units of quote asset. The trade executes at the price:

$$
p = \frac{b_{\mathrm{bid}}}{a_{\mathrm{bid}}}
$$

For convenience, let's denote:

$$
\alpha = A + a_{\mathrm{bid}}
$$

$$
\beta = \frac{B - b_{\mathrm{bid}}}{R} = \frac{B - p a_{\mathrm{bid}}}{R}
$$

Essentially, $\alpha$ is the pool's base asset reserve after the trade, $\beta$ is the quote asset reserve after the trade, adjusted for the oracle price.

The invariant must also hold after the trade:

$$
f(A + a_{\mathrm{bid}}, B - b_{\mathrm{bid}}) = \alpha^3 \beta + \alpha \beta^3 = K
$$

This is a 4th-degree (quartic) equation, so finding a closed form solution is not feasible. Instead, we solve it by **Newton's method**. Define:

$$
g(a_{\mathrm{bid}}) = \alpha^3 \beta + \alpha \beta^3 - K
$$

We must solve for $a_{\mathrm{bid}}$ such that $g(a_{\mathrm{bid}}) = 0$. This can be done by:

- $a_0 \gets A$
- for $n = 1, 2, 3, \dots$:
  - $a_n \gets a_{n-1} - \frac{g(a_{n-1})}{g'(a_{n-1})}$
  - if converge, return $a_n$ as the solution for $a_{\mathrm{bid}}$

where

$$
g'(a_{\mathrm{bid}}) = -\frac{p}{R} \alpha^3 + a \alpha^2 \beta - \frac{3p}{R} \alpha \beta^2 + \beta^3
$$

The choice of the initial value $a_0$ is important. This is because $g(a_{\mathrm{bid}}) = 0$ has a trivial solution of $a_{\mathrm{bid}} = 0$, which corresponding to not placing an order at all. Instead, we want to find the non-trivial solution of $a_{\mathrm{bid}} > 0$. Emprically, choosing $a_0 = A$ always gives us the intended solution.

For the SELL side, we do exactly the same, except for:

$$
\alpha = A - a_{\mathrm{ask}}
$$

$$
\beta = \frac{B + b_{\mathrm{ask}}}{R} = \frac{B + p a_{\mathrm{ask}}}{R}
$$

$$
g'(a_{\mathrm{ask}}) = \frac{p}{R} \alpha^3 - a \alpha^2 \beta + \frac{3p}{R} \alpha \beta^2 - \beta^3
$$

#### Example

Following the same example of SOL-USD pool in the previous discussions, assuming oracle price $R = 200$ (USD per SOL), the liquidity depth can be computed and plotted as follows:

![Order size and depth following the Solidly invariant](2-solidly.png)

It's obvious that liquidity is now indeed concentrated around the pool price.

In general, however, $R$ does not match exactly the composition of assets in the pool. In order to mitigate LVR, we need $p_{\mathrm{pool}}$ to closely track $R$. Let's plot the deviation of $p_{\mathrm{pool}}$ from $R$ against $R$:

![deviation of pool price from oracle price](3-solidly-pool-price-vs-oracle.png)

As seen, the deviation does not exceed 0.003% when oracle price jumps less than ~5%. As such, we believe the pool is not ssusceptible to LVR, given that the oracle price's latency is sufficient low to reflect the true asset price.

### Minimizing LVR with oracle-imformed CFMM

In order for the passive liquidity pool to provide accurate quotes, it's important that:

- it has access to a low-latency oracle that accurately reports the up-to-date market prices; and,
- it must be able to adjust its orders before any other trader is able to pick up the scale quotes.

For the oracle, we propose using [Pyth Lazer](https://x.com/PythNetwork/status/1879545169781166397), which provides ultra fast price feeds with 1 millisecond updates. Each block, the proposer fetches the prices and submit them onchain in a transaction on the very top of the block.

To ensure the passive liquidity pool has priority in updating quotes, we utilize a technique which we dub **virtual order book**. Instead of physically placing orders, the pool constructs a virtual order book based its asset reserves and oracle price, prior to any order being matched. The matching engine then combine the real, physical order book with the virtual book before performing matching. See Appendix A for code snippets demonstrating this idea.

### Mitigating MEV with frequent batch auctions

A searcher relies on two prerequisites to both be satisfied in order to pull off malicious MEV activities:

- user orders are broadcasted transparently, unencrypted, through the public p2p network;
- trades are possibly executed at different prices throughout the same block.

On the first point, we believe the endgame solution is [encrypted mempool](https://eprint.iacr.org/2022/898). However, to achieve a faster go-to-market, we propose a faster interim solution: the Dango blockchain to run on a proof-of-authority validator set; validators to configure their mempools such that they only receive transactions, and broadcast only to other validators, but not to any other node. Users will broadcast their transactions directly to the validators. As such, assume validators themselves do not engage in malicious MEV (as they will be contractually obliged to), user transactions can be considered private.

On the second point, we propose to execute orders using the [frequent batch auction](https://academic.oup.com/qje/article/130/4/1547/1916146) approach. Specifically, the flow of a block would be:

1. In the first transaction, the proposer submits up-to-date oracle prices.
2. Users submit orders. These orders are stored in a **transient storage** in the DEX smart contract. Data in the transient storage only persists through one block (wiped at the end of the block) and importantly, cannot be queried by other contracts.
3. After all user transactions have been processed, the DEX contract iterates through all trading pairs that have received new orders in this block and attempts to match the orders:

   1. Add new orders in the transient storage into the order book.
   2. Compute the virtual order book of the passive liquidity pool based the pool's asset reserves and oracle price.
   3. Combine the two books in (i) and (ii) and perform order matching through a [uniform price auction](https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better). The objective is to find the intersection between the aggregate supply and demand curves, which is the price that maximizes trading volume.
   4. Given the clearing price found in (iii), fill the orders, send the output funds to traders.

There are two notable elements in this:

- This is a **sealed-bid** auction. Given that users broadcast transactions to a private mempool, and orders are stored in the transient storage, the orders are kept confidential prior to the auction.
- This is a **uniform price** auction. All trades are settled at the same clearing price, so it's literally impossible to execute sandwich attacks.

## Conclusion

We have identified, analyzed the causes of, and proposed Dango DEX as a solution to the following problems:

| The problem                                                  | The cause                                                                                                  | Our solution                                                                                                           |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Market making on LOBs is not accessible for retail investors | High level of sophistication is required                                                                   | Our LOB to be enshrined with a passive liquidity pool that follows the Solidly AMM curve                               |
| LVR                                                          | AMMs do not actively adjust quotes in reaction to changes in asset prices                                  | Our passive liquidity pool to adjust its curve based on a low-latency oracle feed, with priority over any other trader |
| MEV                                                          | User orders are broadcasted publicly, and are settled at potentially different prices throughout the block | Private mempool; frequent sealed-bid batch auctions at uniform prices                                                  |

## Appendix A. Suggested implementation

We suggest performing FBA at the frequency of once per block. Research suggests the optimal frequency for FBA is 0.2–0.9 second; we suggest picking a block time within this range.

Each block should follow the following workflow:

1. At the beginning of the block, validators submit an up-to-date oracle feed of supported assets.
2. During the block, the DEX smart contract accepts user orders and save them in the book, but do nothing else with the orders.
3. At the very end of the block, the DEX matches and clears the orders.

To ensure the auctions to be sealed-bid, new orders accepted om step (2) must not be made public for query by other smart contracts. Only after the auction is completed, unfilled orders are made public.

The specific algorithm for matching the orders is described below, in Rust pseudocode.

First, we define the following types:

```rust
enum Directioin {
    Bid,
    Ask,
}
```

```rust
struct Order {
    /// The order's limit price.
    pub price: Decimal,
    /// The quantity of that's not yet filled, measured in the base asset.
    pub quantity: Uint128,
}
```

Second, the DEX contract must be capable of iterating over orders in the book following the **price-time priority**. That is, orders with better prices (for BUY orders, the higher; for SELL orders, the lower) come first; for orders with the same price, the older ones come first. We abstract this as a Rust iterator type:

```rust
type OrderIterator<'a> = Box<dyn Iterator<Item = Order> + 'a>;
```

Third, the passive liquidity pool must implement the following trait:

```rust
trait LiquidityPool {
    /// Return an iterator over the BUY orders that the passive liquidity pool
    /// would place in the order book following its CFMM invariant, given the
    /// latest oracle price.
    fn get_bids<'a>(&'a self, oracle_price: Decimal) -> OrderIterator<'a>;

    /// Similarly, return an iterator over SELL orders.
    fn get_asks<'a>(&'a self, oracle_price: Decimal) -> OrderIterator<'a>;
}
```

When matching orders, we work with two order books:

- a "**physical**" book that contains orders submitted by users;
- a "**virtual**" book that contains orders that the passive liquidity pool would place.

We must match orders from the two books at the same time. For this we can use a the following merged iterator:

```rust
struct MergedIterator {
    a: Peekable<Item = Order>,
    b: Peekable<Item = Order>,
    direction: Direction,
}

impl MergedIterator {
    pub fn new<A, B>(a: A, b: B) -> Self
    where
        A: Iterator<Item = Order>,
        B: Iterator<Item = Order>,
    {
        Self {
            a: a.peekable(),
            b: b.peekable(),
        }
    }
}

impl Iterator<Item = Order> for MergedIterator {
    fn next(&mut self) -> Option<Order> {
        match (self.a.peek(), self.b.peek()) => {
            // Both iterators have orders left. Return the one with better price.
            // If prices are the, we arbitrarily choose the order in B.
            (Some(a), Some(b)) => {
                match self.direction {
                    Direction::Bid if a.price > b.price | Direction::Ask if a.price < b.price => {
                        a.next()
                    },
                    _ => b.next(),
                }
            },
            // A still has orders, but B has run out. Return the next order in A.
            (Some(_a), None) => a.next(),
            // B still has orders, but A has run out. Return the next order in B.
            (None, Some(_b)) => a.next(),
            // Both iterators have run out of orders. We're done.
            (None, None) => None,
        }
    }
}
```

The DEX smart contract should prepare four iterators:

1. BUY orders in the physical book
2. SELL orders in the physical book
3. BUY ordres in the virtual book
4. SELL orders in the virtual book

and combine them using `MergedIterator`:

```rust
let physical_bid_iter: OrderIterator = /* ... */;
let physical_ask_iter: OrderIterator = /* ... */;

let virtual_bid_iter: OrderIterator = /* ... */;
let virtual_bid_iter: OrderIterator = /* ... */;

// Use the virtual book iterators as the `B` in `MergedIterator`, such that
// orders from the passive liquidity pool is prioritized.
let bid_iter = MergedIterator::new(physical_bid_iter, physical_ask_iter);
let ask_iter = MergedIterator::new(virtual_bid_iter, virtual_bid_iter);
```

Note that we compute the virtual orders based on the latest oracle price _before orders are matched_. This ensures the passively liquidity pool always quotes the latest price, providing its LVR resistance.

Then, we use the following pure function to find the clearing price:

```rust
struct MatchingOutcome {
    /// The range of prices that results in the maximal volume.
    /// The clearing price can be chosen as any value within this range.
    /// It's up to the caller to make the choice.
    range: Option<(Decimal, Decimal)>,
    /// The maximal volume.
    volume: Uint128,
    /// List of BUY orders that have found a match.
    bids: Vec<Order>,
    /// List of SELL orders that have found a match.
    asks: Vec<Order>,
}

fn clear_orders(mut bid_iter: OrderIterator, mut ask_iter: OrderIterator) -> MatchingOutcome {
    let mut maybe_bid = bids.next();
    let mut bids = Vec::new();
    let mut bid_is_new = true;
    let mut bid_volume = Uint128::ZERO;
    let mut maybe_ask = asks.next();
    let mut asks = Vec::new();
    let mut ask_is_new = true;
    let mut ask_volume = Uint128::ZERO;
    let mut range = None;

    loop {
        // If we run out of orders on either side, then we're done.
        let (Some(bid), Some(ask)) = (maybe_bid, maybe_ask) else {
            break;
        }

        // If the prices don't cross, then we're done.
        if bid.price < ask.price {
            break;
        }

        range = Some((ask.price, bid.price));

        if bid_is_new {
            bids.push(bid);
            bid_volume += bid.quantity;
        }

        if ask_is_new {
            asks.push(ask);
            ask_volume += ask.quantity;
        }

        if bid_volume <= ask_volume {
            bid = bid_iter.next();
            bid_is_new = true;
        } else {
            bid_is_new = false;
        }

        if ask_volume <= bid_volume {
            ask = ask_iter.next();
            ask_is_new = true;
        } else {
            ask_is_new = false;
        }
    }

    let volume = bid_volume.min(ask_volume);

    MatchingOutcome { range, volume, bids, asks }
}
```

Once we have the clearing price, clearing the orders is trivial (which involves updating the order state in the book and refund assets to traders) so don't discuss them here.

### Customizations

In the above formulation of the passive liquidity pool, the pool places orders every tick, starting on tick above and below $p_{\mathrm{pool}}$. This can be customized by introducing two additional parameters:

- **Cadence**: The pool can instead place an order every $N$ ticks. Alternatively, it can place orders more densely closer to $p_{\mathrm{pool}}$, but more loosely when farther away. Either way, there will be fewer orders overall, reducing computation load.
- **Spread**: The pool can place orders a few ticks further away from $p_{\mathrm{pool}}$. A bigger spread may improve the pool's profitability.The spread can either be defined as constant values, or calculated dynamically based on marketing conditions, following models such as the one proposed by [Avellaneda and Stoikov](https://people.orie.cornell.edu/sfs33/LimitOrderBook.pdf)

### Dango's custom smart contract VM

We recognize the above is challenging to implement in legacy virtual machines (VMs) such as the Ethereum virtual machine (EVM), due to:

- In EVM, smart contract actions need to be triggered via transactions. This means each block must have a txn at the very beginning of the block to update the oracle price, and another one at the end of the block to trigger order matching. During periods of high traffic, it can be difficult to get txns into the block, not to mention ensuring they are located at the beginning and end of the block. The only way to achieve this is to utilize a centralized block builder, which introduced additional trust assumptions.

  Dango's custom smart contract VM, [Grug](https://grug.build/whitepaper.html), allows validators to submit oracle updates via Tendermint's ABCI++ API, at the top of every block. Additionally, it automatically triggers order matching via end-of-block cronjobs.

- EVM does not support iterating keys in its `mapping` data structures. This is because in EVM, the state of each contract is a hash map. Since the map keys are hashed, they are essentially randomized and thus cannot be iterated.

  In Grug, contract states are B-tree maps which support iteration.
