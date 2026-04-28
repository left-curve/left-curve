---
paths:
  - "ui/**/*.{ts,tsx}"
---
# UI Rules (React 19 + Zustand + Composition)

## Component Architecture (Composition Patterns)

Never add boolean props to customize behavior — use composition.
Each boolean doubles possible states and creates unmaintainable conditionals.

### Compound Components

Structure complex components with shared context and sub-components.
Consumers compose the pieces they need.

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
  const { state, actions: { update }, meta: { inputRef } } = use(ComposerContext);
  return (
    <TextInput
      ref={inputRef}
      value={state.input}
      onChangeText={(text) => update((s) => ({ ...s, input: text }))}
    />
  );
}

// Export as compound component
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
</Composer.Provider>
```

### Explicit Variants Over Boolean Props

```tsx
// BAD — what does this render?
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
    <Composer.Provider state={state} actions={{ update: setState, submit: forwardMessage }}>
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

### Lift State into Providers

Move state into provider components for sibling access. Components that need
shared state don't have to be visually nested — just within the provider.

```tsx
// Components OUTSIDE Composer.Frame can still access state
function ForwardMessageDialog() {
  return (
    <ForwardMessageProvider>
      <Dialog>
        <Composer.Frame>
          <Composer.Input />
        </Composer.Frame>
        <MessagePreview />        {/* reads composer state */}
        <ForwardButton />         {/* calls composer submit */}
      </Dialog>
    </ForwardMessageProvider>
  );
}
```

### Children Over Render Props

Use `children` for composition, not `renderX` props. Render props only when
parent needs to provide data back to child (e.g., list `renderItem`).

```tsx
// BAD
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

- `use()` instead of `useContext()` — can be called conditionally
- No `forwardRef` — `ref` is a regular prop in React 19
- Plain function components preferred over `React.FC`

```tsx
// BAD (React 18)
const Input = forwardRef<HTMLInputElement, Props>((props, ref) => {
  return <input ref={ref} {...props} />;
});
const value = useContext(MyContext);

// GOOD (React 19)
function Input({ ref, ...props }: Props & { ref?: React.Ref<HTMLInputElement> }) {
  return <input ref={ref} {...props} />;
}
const value = use(MyContext);
```

## State Management: Zustand Only

All application state lives in Zustand stores. No `useState`/`useRef`/`useReducer`
for state that is shared, subscribed, buffered, or persisted.

### When to use what

| Tool | Use for |
|------|---------|
| Zustand store | Global state, subscriptions, real-time data, persisted state, shared across components |
| React Context | Compound component communication only (provider/consumer for composition) |
| Local `useState` | Truly ephemeral UI state (hover, focus, input being typed) |

### Zustand Patterns

```tsx
// Selectors for render — components only re-render on selected changes
const mode = TradePairStore((s) => s.mode);
const pairId = TradePairStore((s) => s.pairId);

// Actions via store methods
TradePairStore.getState().setPair(pairId, type);
```

- `.getState()` only in event handlers/callbacks outside React render cycle
- Selectors in components for reactive updates
- Subscription logic (debounce, buffer, dedup) belongs in store actions/middleware,
  not in useRef/useEffect inside hooks
- `subscribeWithSelector` middleware for fine-grained subscriptions

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
- `classNames` object prop for fine-grained nested styling
- `compoundVariants` for state combinations
- Never inline style objects

## Data Fetching

- TanStack React Query for server state (`useQuery`, `useMutation`)
- Query keys structured: `["domain", ...params]`
- `enabled` flag for conditional queries
- `queryClient` passed through router context

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
