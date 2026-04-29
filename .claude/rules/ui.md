---
paths:
  - "ui/**/*.{ts,tsx}"
---

# UI Rules (React 19 + Zustand + Composition)

## Component Architecture (Composition Patterns)

Prefer composition over boolean-prop proliferation.
Standard boolean props (`disabled`, `loading`, `open`, `required`) are fine.
The anti-pattern is multiple booleans that create invalid state combinations.

### Compound Components

Structure complex components with shared context and sub-components.
Consumers compose the pieces they need. Compound component families
may export multiple related components from a single module.

```tsx
const ComposerContext = createContext<ComposerContextValue | null>(null);

function ComposerProvider({ children, state, actions, meta }: ProviderProps) {
  return (
    <ComposerContext value={{ state, actions, meta }}>
      {children}
    </ComposerContext>
  );
}

function ComposerInput() {
  const {
    state,
    actions: { update },
    meta: { inputRef },
  } = use(ComposerContext);
  return (
    <TextInput
      ref={inputRef}
      value={state.input}
      onChangeText={(text) => update((s) => ({ ...s, input: text }))}
    />
  );
}

// Export as compound component — multiple exports from one module is fine here
const Composer = {
  Provider: ComposerProvider,
  Frame: ComposerFrame,
  Input: ComposerInput,
  Submit: ComposerSubmit,
};

// Usage
<Composer.Provider state={state} actions={actions} meta={meta}>
  <Composer.Frame>
    <Composer.Input />
    <Composer.Submit />
  </Composer.Frame>
</Composer.Provider>;
```

### Explicit Variants Over Boolean-Prop Proliferation

```tsx
// BAD — invalid state combinations possible
<Composer isThread isEditing={false} showAttachments />

// GOOD — self-documenting
<ThreadComposer channelId="abc" />
<EditMessageComposer messageId="xyz" />
```

Each variant composes shared parts but is explicit about what it renders,
what provider/state it uses, and what actions are available.

### Context Interface for Dependency Injection

Define generic interface with `state`, `actions`, `meta`. Provider is the only
place that knows how state is managed — UI components consume the interface.

```tsx
interface ComposerContextValue {
  state: ComposerState;
  actions: ComposerActions;
  meta: ComposerMeta;
}
```

Different providers implement the same interface — same UI works with all:

```tsx
// Local state for ephemeral forms
function ForwardMessageProvider({ children }) {
  const [state, setState] = useState(initialState);
  const forwardMessage = useForwardMessage();
  return (
    <Composer.Provider
      state={state}
      actions={{ update: setState, submit: forwardMessage }}
    >
      {children}
    </Composer.Provider>
  );
}

// Global synced state for channels
function ChannelProvider({ channelId, children }) {
  const { state, update, submit } = useGlobalChannel(channelId);
  return (
    <Composer.Provider state={state} actions={{ update, submit }}>
      {children}
    </Composer.Provider>
  );
}
```

### State Lifting

Keep state as local as possible. Lift into providers only when multiple
consumers actually need coordinated access. Provider-heavy design increases
render surface and complexity — lift when justified, not by default.

```tsx
// Components OUTSIDE Composer.Frame can still access state
function ForwardMessageDialog() {
  return (
    <ForwardMessageProvider>
      <Dialog>
        <Composer.Frame>
          <Composer.Input />
        </Composer.Frame>
        <MessagePreview /> {/* reads composer state */}
        <ForwardButton /> {/* calls composer submit */}
      </Dialog>
    </ForwardMessageProvider>
  );
}
```

### Children Over Render Props

Prefer `children` for composition. Render props remain valid for headless
composition, cell renderers, and cases where the parent must provide data
back to the child (e.g., list `renderItem`, table cell renderers).

```tsx
// BAD — when children would suffice
<Composer renderHeader={() => <CustomHeader />} renderFooter={() => <Footer />} />

// GOOD
<Composer.Frame>
  <CustomHeader />
  <Composer.Input />
  <Composer.Footer>
    <Composer.Submit />
  </Composer.Footer>
</Composer.Frame>
```

## React 19

- `ref` is a regular prop — no `forwardRef`
- Plain function components preferred over `React.FC`
- `useActionState` for form submission lifecycle (pending/error/success)
- `useContext` for standard context reads; `use()` when conditional/iterative
  context reads or promise unwrapping is needed

```tsx
// BAD (React 18)
const Input = forwardRef<HTMLInputElement, Props>((props, ref) => {
  return <input ref={ref} {...props} />;
});

// GOOD (React 19)
function Input({
  ref,
  ...props
}: Props & { ref?: React.Ref<HTMLInputElement> }) {
  return <input ref={ref} {...props} />;
}
```

## Prop Design

Use discriminated union props for components with mutually exclusive modes.
This makes invalid prop combinations unrepresentable.

```tsx
// BAD — what happens when both are provided?
type Props = { userId?: string; address?: string };

// GOOD — one mode or the other, enforced by types
type Props =
  | { mode: "user"; userId: string }
  | { mode: "address"; address: string };
```

Restrict prop types to the narrowest possible:

- `"sm" | "md" | "lg"` over `string`
- `readonly Coin[]` over `Coin[]`
- `Decimal` over `number` for financial values

## Custom Hooks

- Prefix with `use` (enforced by React's linting rules)
- Single responsibility — one hook, one concern
- Return shape should follow ergonomics for the use case
- Hooks that wrap TanStack Query should follow `useXxx` naming: `useBalances`, `usePool`
- Colocate hooks with the feature they serve, not in a global `/hooks` directory
- Don't extract trivial hooks — a hook wrapping one `useState` + one `useEffect`
  is probably not worth the indirection

```tsx
// Feature-scoped hook — colocated with trade feature
function useTradeEstimate(params: TradeParams) {
  return useQuery({
    queryKey: ["trade", "estimate", params],
    queryFn: () => client.estimateSwap(params),
    enabled: params.amount.gt(0),
  });
}
```

## State Management

### Layer Model

| Layer         | Tool             | Use for                                                                                            |
| ------------- | ---------------- | -------------------------------------------------------------------------------------------------- |
| Server state  | TanStack Query   | All fetched data, mutations, cache                                                                 |
| Client state  | Zustand store    | Global state, subscriptions, real-time data, persisted state, shared across components             |
| Cross-cutting | React Context    | Compound component communication, theme, i18n, auth/session, feature flags, stable service objects |
| Ephemeral     | Local `useState` | Truly ephemeral UI state (hover, focus, input being typed)                                         |

Server state and client state are fundamentally different — never store
fetched data in Zustand. Use TanStack Query for anything from the server.
Avoid putting high-frequency market data in React Context — it re-renders all consumers.

### Zustand Patterns

```tsx
// Selectors for render — components only re-render on selected changes
const mode = TradePairStore((s) => s.mode);
const pairId = TradePairStore((s) => s.pairId);

// Actions via store methods
TradePairStore.getState().setPair(pairId, type);
```

- `.getState()` only in event handlers/callbacks outside React render cycle
- Selectors in components for reactive updates — never subscribe to the entire store
- Use `shallow` equality for selectors returning objects/arrays in high-frequency components
- Subscription logic (debounce, buffer, dedup) belongs in store actions/middleware,
  not in useRef/useEffect inside hooks
- `subscribeWithSelector` middleware for fine-grained subscriptions

### Persisted State Hygiene

When using Zustand `persist` middleware:

- Version the persisted shape (`version` field) and provide `migrate` functions
- Persist only what's needed — use `partialize` to exclude transient fields
- Set TTLs for data that goes stale (balances, quotes, allowances)
- Never persist raw financial data (balances, prices) without TTL — stale
  financial state after reload can mislead users into unsafe actions

```tsx
persist(storeCreator, {
  name: "trade-settings",
  version: 2,
  partialize: (state) => ({ pairId: state.pairId, slippage: state.slippage }),
  migrate: (persisted, version) => {
    /* handle shape changes */
  },
});
```

## Effects

Use `useEffect` only to synchronize with external systems (subscriptions,
DOM APIs, WebSockets, third-party widgets). Never use `useEffect` for:

- Deriving state from props/state — compute during render instead
- Transforming data from queries — use TanStack Query's `select`
- Responding to events — use event handlers

```tsx
// BAD — useEffect for derivation
const [fullName, setFullName] = useState("");
useEffect(() => {
  setFullName(`${firstName} ${lastName}`);
}, [firstName, lastName]);

// GOOD — compute during render
const fullName = `${firstName} ${lastName}`;
```

## Performance

Profile first, optimize second. Never add memoization without measuring.

### Memoization Guidelines

- Do not add `React.memo`, `useMemo`, or `useCallback` by default
- `React.memo`: wrap components that receive stable props but re-render due to parent
  (list items, table rows, chart elements with many siblings)
- `useMemo`: expensive computations only (sorting/filtering large datasets, complex math).
  If it works without it, leave it out
- `useCallback`: only when passing callbacks to memoized children — otherwise pointless
- Never use array index as `key` for dynamic lists — use a stable identifier

### Context Performance

Split contexts by update frequency. A single monolithic context that changes on
every keystroke will re-render every consumer.

```tsx
// BAD — all consumers re-render when any value changes
const AppContext = createContext({ theme, locale, user, notifications });

// GOOD — separate by update frequency
const ThemeContext = createContext(theme);
const NotificationContext = createContext(notifications);
```

### Expensive Renders

- Virtualize long lists (TanStack Virtual)
- Debounce high-frequency inputs before triggering queries
- Use `startTransition` for non-urgent state updates

## Error Handling

### Error Boundaries

Place error boundaries at logical widget boundaries — not one per app,
not one per component. Granularity should match "what can recover independently."

```tsx
<ErrorBoundary fallback={<TradeWidgetError />}>
  <Suspense fallback={<TradeWidgetSkeleton />}>
    <TradeWidget />
  </Suspense>
</ErrorBoundary>
```

Error boundaries do NOT catch: async errors, event handler errors, SSR errors.
Use try/catch for those.

### Suspense

Pair `Suspense` with `ErrorBoundary` at feature boundaries.
Use meaningful skeleton UIs, not generic spinners.

```tsx
<Suspense fallback={<OrderBookSkeleton />}>
  <OrderBook pairId={pairId} />
</Suspense>
```

## Component Organization

### File Structure

Colocate everything related to a feature: component, hooks, types, tests, styles.

```
features/
  trade/
    TradeWidget.tsx
    TradeWidget.spec.tsx
    useTradeEstimate.ts
    tradeVariants.ts
    types.ts
```

### Component File Convention

One primary exported component per file. Exception: compound component families
may export multiple related components from a single module when they share
internal context or are tightly coupled.

## Accessibility Baseline

- All interactive elements must be keyboard accessible
- Images require `alt` text (empty `alt=""` for decorative images)
- Form inputs must have associated labels (visible or `aria-label`)
- Focus management: trap focus in modals/dialogs, restore on close
- Use semantic HTML elements (`button`, `nav`, `main`, `dialog`) over generic `div`
- Color contrast: meet WCAG AA (4.5:1 for normal text, 3:1 for large text)
- `aria-live` regions for dynamic content updates (toasts, notifications)
- Respect `prefers-reduced-motion` — disable or simplify Framer Motion animations
  for users who request reduced motion

```tsx
const prefersReducedMotion = window.matchMedia(
  "(prefers-reduced-motion: reduce)",
).matches;

<motion.div
  animate={{ opacity: 1 }}
  transition={prefersReducedMotion ? { duration: 0 } : { duration: 0.3 }}
/>;
```

## Styling

tailwind-variants (tv) with slot-based variants for multi-part components:

```typescript
const inputVariants = tv({
  slots: {
    base: "flex flex-col gap-1",
    wrapper: "px-4 py-3 rounded-lg bg-surface-secondary",
    input: "flex-1 bg-transparent outline-none",
  },
  variants: {
    isInvalid: { true: { wrapper: "border-red-500" } },
  },
});

const { base, wrapper, input } = inputVariants({ isInvalid });
```

- `twMerge` for className composition
- `classNames` objects prop for fine-grained nested styling
- `compoundVariants` for state combinations
- Prefer class-based styling by default. Inline `style` objects are acceptable for
  CSS custom properties, computed transforms/dimensions, and Framer Motion values

## Data Fetching

- TanStack React Query for all server state (`useQuery`, `useMutation`)
- Query keys structured: `["domain", ...params]` as `readonly` tuple

### Query Key Factories

Centralize query keys per domain to prevent key drift:

```typescript
const dexKeys = {
  all: ["dex"] as const,
  pools: (params?: PoolParams) => [...dexKeys.all, "pools", params] as const,
  pool: (poolId: string) => [...dexKeys.all, "pool", poolId] as const,
};
```

### Query Options

- `enabled` flag for conditional queries
- `queryClient` passed through router context
- Use `select` to derive/transform query data instead of `useMemo` around query results
- Prefetch with `ensureQueryData` in route loaders for instant navigation

### Mutations

- Update cache directly from the mutation result (`onMutate`/`onSuccess` + `setQueryData`)
  when you can reconstruct the new state locally
- Invalidate selectively when the server may have changed data you cannot reconstruct
- Avoid blanket `invalidateQueries` — it causes unnecessary refetch churn

```tsx
const { data: topPools } = useQuery({
  queryKey: dexKeys.pools({ limit: 10 }),
  queryFn: () => client.getPools({ limit: 10 }),
  select: (pools) => pools.filter((p) => p.tvl.gt(Decimal(1000))),
});
```

### Optimistic Updates

For server-state optimistic updates, use TanStack Query's `onMutate` + cache rollback
pattern. This keeps the query cache as the single source of truth for server data.

```tsx
useMutation({
  mutationFn: submitOrder,
  onMutate: async (newOrder) => {
    await queryClient.cancelQueries({ queryKey: orderKeys.all });
    const previous = queryClient.getQueryData(orderKeys.open());
    queryClient.setQueryData(orderKeys.open(), (old) => [
      ...(old ?? []),
      newOrder,
    ]);
    return { previous };
  },
  onError: (_err, _vars, context) => {
    queryClient.setQueryData(orderKeys.open(), context?.previous);
  },
  onSettled: () => {
    queryClient.invalidateQueries({ queryKey: orderKeys.all });
  },
});
```

## WebSocket / Real-Time Data

### Snapshot + Delta Reconciliation

Order books and live data feeds must handle:

- **Sequence numbers**: track and detect gaps
- **Gap detection**: request full resync on missed sequences
- **Deduplication**: ignore already-applied deltas
- **Stale detection**: surface data age to the UI when feeds go silent

```tsx
function applyDelta(book: OrderBook, delta: BookDelta): OrderBook {
  if (delta.sequence <= book.lastSequence) return book; // already applied
  if (delta.sequence > book.lastSequence + 1) {
    requestResync(); // gap detected
    return book;
  }
  return { ...book, ...mergeDelta(book, delta), lastSequence: delta.sequence };
}
```

### Freshness Metadata

Prices, quotes, balances, and PnL should carry freshness metadata
(block height, timestamp, quote expiry). Surface staleness in the UI
and reject expired quotes before submission.

```tsx
type PriceFeed = {
  price: Decimal;
  blockHeight: bigint;
  timestamp: number;
  expiresAt: number;
};

function isStale(feed: PriceFeed): boolean {
  return Date.now() > feed.expiresAt;
}
```

## Transaction State Machine

Trades, LP operations, approvals, bridges, and withdrawals follow a state machine:

```typescript
type TxStatus =
  | { status: "idle" }
  | { status: "signing" }
  | { status: "broadcasting"; signedTx: Uint8Array }
  | { status: "pending"; txHash: TxHash }
  | { status: "confirmed"; txHash: TxHash; blockHeight: bigint }
  | { status: "failed"; txHash: TxHash; error: BaseError }
  | { status: "replaced"; oldHash: TxHash; newHash: TxHash };
```

- Key by txHash or client-generated orderId for deduplication
- Deduct spendable balance on `pending`, restore on `failed`
- Never treat `pending` as final — wait for confirmation at the appropriate finality threshold
- Provide rollback logic for each optimistic state transition

## Routing

- TanStack Router (file-based) with `createFileRoute()`
- Lazy routes with `.lazy.tsx` suffix
- Search params validated with Zod: `validateSearch: z.object({...})`

## Forms

`useInputs` hook for form state with validation/masking:

```typescript
const { register, handleSubmit, setValue } = useInputs({
  initialValues: { email: "" },
});

const emailField = register("email", {
  strategy: "onBlur",
  validate: (v) => isValidEmail(v) || "Invalid email",
  mask: (v) => v.toLowerCase(),
});

<form onSubmit={handleSubmit(({ email }) => submit(email))}>
  <Input {...emailField} />
</form>
```

## Animation

Framer Motion with `AnimatePresence` for mount/unmount:

```tsx
<AnimatePresence mode="wait">
  <motion.div
    key={id}
    initial={{ opacity: 0, y: -20 }}
    animate={{ opacity: 1, y: 0 }}
    exit={{ opacity: 0, y: 20 }}
    transition={{ duration: 0.3 }}
  >
    {content}
  </motion.div>
</AnimatePresence>
```

Polymorphic: `<Button as={motion.div} isLoading={isPending}>{label}</Button>`

## Event Handlers

- Props: `on` prefix (`onChange`, `onSubmit`, `onSuccess`)
- Inline composition with array syntax: `onClick={() => [navigate({ to: "/bridge" }), setSidebarVisibility(false)]}`

## I18N

Paraglide: `m["key"]()` pattern

```tsx
<h1>{m["common.signin"]()}</h1>
<span>{m["bridge.network"]({ network })}</span>
```
