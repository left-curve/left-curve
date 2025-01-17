# Nonces and unordered transactions

**Nonce** is a mechanism to prevent **replay attacks**.

Suppose Alice sends 100 coins to Bob on a blockchain that doesn't employ such a mechanism. An attacker can observe this transaction (tx) confirmed onchain, then broadcasts it again. Despite the second time this tx is not broadcasted by Alice, it does contain a valid signature from Alice, so it will be accepted again. Thus, total 200 coins would leave Alice's wallet, despite she only consents to sending 100. This can be repeated until all coins are drained from Alice's wallet.

To prevent this,

- each tx should include a nonce, and
- the account should internally track the nonce it expects to see from the next tx.

The first time an account sends a tx, the tx should include a nonce of $0$; the second time, $1$; so on. Suppose Alice's first tx has a nonce of $N$. If the attacker attempts to broadcast it again, the tx would be rejected by the mempool, because Alice's account expects a nonce of $N + 1$.

> The above describes **same-chain replay attack**. There is also **cross-chain replay attack**, where an attacker observes a tx on chain A, and broadcasts it again on chain B. To prevent this, transactions include a **chain ID** besides nonce.

## The problem

The drawback of this naïve approach to handling nonces is _it enforces a strict ordering of all txs_, which doesn't do well in use cases where users are expected to submit txs with high frequency. Consider this situation:

- Alice's account currently expects a nonce of $N$;
- Alice sends a tx (let's call this tx A) with nonce $N$;
- Alice immediately sends another tx (B) with nonce $N + 1$;
- due to network delays, tx B arrives on the block builder earlier than A.

Here, the block builder would reject tx B from entering the mempool, because it expects a nonce of $N$, while tx B comes with $N + 1$. When tx A later arrives, it will be accepted. The result is Alice submits two txs, but only one makes it into a block.

Imagine Alice is trading on an order book exchange and wants to cancel two active limit orders. These actions are not correlated – there's no reason we must cancel one first then the other. So Alice click buttons to cancel the two in quick succession. However, only one ends up being canceled; she has to retry canceling the other one. Bad UX!

## HyperLiquid's solution

As described [here](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/nonces-and-api-wallets).

In HyperLiquid, an account can have many **session keys**, each of which has its own nonce. In our case, to simplify things, let's just have one nonce for each account (across all session keys).

Instead of tracking a single nonce, _the account tracks the most recent $X$ nonces it has seen_ (let's call these the `SEEN_NONCES`). HyperLiquid uses $X = 20$, while for simplicity in the discussion below let's use $X = 5$.

Suppose Alice's account has the following `SEEN_NONCES`: $[5, 6, 7, 9, 10]$. $8$ is missing because it got lost due to network problems.

Now, Alice broadcasts two txs in quick succession, with nonces $11$ and $12$. Due to network delays, $12$ arrives at the block builder first.

The account will carry out the following logic:

- **accept the tx if its nonce is newer than the oldest nonce in `SEEN_NONCES`, and not already in `SEEN_NONCES`**;
- insert the tx's nonce into `SEEN_NONCES`.

When $12$ arrives first, it's accepted, and `SEEN_NONCES` is updated to: $[6, 7, 9, 10, 12]$. ($5$ is removed because we only keep the most recent $X = 5$ nonces.)

When $11$ arrives later, it's also accepted, with `SEEN_NONCES` updated to: $[7, 9, 10, 11, 12]$.

This solves the UX problem we mentioned in the previous section.

## Transaction expiry

Now suppose tx $8$ finally arrives. Since it was created a long while ago, it's most likely not relevant any more. However, following the account's logic, it will still be accepted.

To prevent this, we should add an `expiry` parameter into the tx metadata. If the expiry is earlier than the current block time, the tx is rejected, regardless of the nonce rule.

`expiry` can be either a block height or timestamp. For Dango's use case, timestamp probably makes more sense.
