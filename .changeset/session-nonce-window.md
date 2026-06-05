---
"@left-curve/sdk": minor
"@left-curve/types": minor
---

Route nonce queries to per-session-key windows. `Client` / `ClientConfig` gain an optional `sessionKey` field; session-backed clients populate it from `SigningSessionInfo`. `signAndBroadcastTx` uses it to call the new `session_seen_nonces` query (with fallback to the standard window's high-water mark when the session window is empty) instead of the shared `seen_nonces`. Adds a `getAccountSessionSeenNonces` action. The `Signer` interface is unchanged.
