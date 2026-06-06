# Live Resources

Subscription-backed live data in `@left-curve/store` uses live resources instead of exported
singleton Zustand stores.

Rules:

- Components read live data through selector hooks: `useX(selector, params, equalityFn?)`.
- Selectors are required. The default selector equality is `Object.is`.
- Hook params carry domain identity explicitly, such as `accountAddress`, `perpsPairId`,
  `bucketSize`, `limit`, and `enabled`.
- Missing params or `enabled: false` means the hook does not acquire the resource and reads the
  idle snapshot.
- Contract addresses and canonical subscription cadence are owned inside each resource hook.
- Resource snapshots are partitioned by params. They are not active-account or active-pair
  singletons.
- The resource primitive owns acquire/release, ref counting, cache policy, status/error, version
  guarding, equality, and metadata-only browser instrumentation.
- Resource equality should compare status/error and stable payload fields. Block-height-only updates
  should not wake selectors unless a resource explicitly exposes block height as meaningful data.
- Runtime listener errors must flow through the resource error callback so route-level consumers can
  surface stale-data risk.
- The React adapter can restart a same-key resource through `restartToken` when runtime handles
  change without changing domain identity.
- Do not split "start subscription" from "read state" for subscription-backed live data.
- Do not export raw stores for this live perps/market data family.

The browser exposes metadata at `window.__DANGO_LIVE_RESOURCES__`. It includes resource names, keys,
ref counts, listener counts, status, versions, and update counts. It intentionally does not include
snapshot payloads.
