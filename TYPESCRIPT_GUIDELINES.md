# TypeScript Patterns and Conventions

Patterns and conventions for `.ts` and `.tsx` files in this monorepo.

---

## Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Source files | camelCase | `baseClient.ts`, `grugClient.ts` |
| React components | PascalCase | `Button.tsx`, `AccountMenu.tsx` |
| Directories | lowercase | `clients/`, `crypto/`, `types/` |
| Test files | `.spec.ts` suffix | `binary.spec.ts` |
| Story files | `.stories.tsx` suffix | `Button.stories.tsx` |

---

## Exports

- **Named exports only** - no default exports
- **Barrel files** (`index.ts`) re-export with explicit types
- **ESM extensions** - always include `.js` in imports

```typescript
// Barrel file pattern
export { createBaseClient } from "./baseClient.js";
export { type GrugClient, createGrugClient } from "./grugClient.js";

// Wildcard only in barrel files
export * from "./webauthn/index.js";
```

---

## Type Definitions

| Use | For |
|-----|-----|
| `interface` | Contracts, structural patterns (`HashFunction`, `KeyPair`) |
| `type` | Unions, aliases, complex types (`Result<T>`, `Message`) |

```typescript
// Interfaces for contracts
interface HashFunction {
  readonly blockSize: number;
  update(data: Uint8Array): HashFunction;
  digest(): Uint8Array;
}

// Types for everything else
type Result<T, E = Error> = Success<T> | Failure<E>;
type Option<T> = T | undefined;

// Generic constraints with defaults
function createClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = undefined,
>(config: Config<transport, chain>): Client<transport, chain>
```

---

## Functions

- **Arrow functions** as standard
- **Async/await** for promises
- **Factory functions** for object creation

```typescript
// Factory pattern
export function createGrugClient<T extends Transport>(
  parameters: GrugClientConfig<T>,
): GrugClient<T> {
  const client = createBaseClient({ ...parameters, type: "grugClient" });
  return client.extend(grugActions);
}
```

---

## Code Style Preferences

### Prefer `const` Over `let`

Always use `const` to prevent mutations. Use `let` only when reassignment is necessary.

```typescript
// ✅ Prefer const
const config = { timeout: 5000 };
const items = [1, 2, 3];
const result = items.map((x) => x * 2);

// ❌ Avoid let when not needed
let data = fetchData();  // If never reassigned, use const
```

### IIFE for Scoped Initialization

Use Immediately Invoked Function Expressions to encapsulate complex initialization logic and improve readability.

```typescript
// Complex conditional assignment
const apiUrl = (() => {
  if (process.env.NODE_ENV === "production") return "https://api.prod.com";
  if (process.env.NODE_ENV === "staging") return "https://api.staging.com";
  return "http://localhost:3000";
})();

// Async IIFE for initialization
const client = await (async () => {
  const config = await loadConfig();
  return createClient(config);
})();

// Scoped logic in components
const formattedData = (() => {
  const sorted = [...data].sort((a, b) => a.value - b.value);
  const filtered = sorted.filter((x) => x.active);
  return filtered.map((x) => ({ ...x, label: x.name.toUpperCase() }));
})();
```

### Immutable Operations

Prefer non-mutating array/object operations.

```typescript
// ✅ Spread for copies
const newArray = [...items, newItem];
const newObject = { ...config, timeout: 3000 };

// ✅ Non-mutating array methods
const sorted = [...items].sort((a, b) => a - b);
const updated = items.map((item) => item.id === id ? { ...item, active: true } : item);
const filtered = items.filter((item) => item.active);

// ❌ Avoid mutations
items.push(newItem);
items.sort();
config.timeout = 3000;
```

### Early Returns for Readability

Exit early to reduce nesting and improve clarity.

```typescript
// ✅ Early returns
function process(data: Data | null) {
  if (!data) return null;
  if (!data.isValid) return { error: "Invalid data" };
  return transform(data);
}

// ❌ Avoid deep nesting
function process(data: Data | null) {
  if (data) {
    if (data.isValid) {
      return transform(data);
    } else {
      return { error: "Invalid data" };
    }
  }
  return null;
}
```

### Destructuring with Defaults

Use destructuring with defaults for cleaner parameter handling.

```typescript
// Function parameters
function createClient({ timeout = 5000, retries = 3 }: ClientOptions = {}) {
  // ...
}

// Object destructuring
const { data = [], isLoading = false } = response;

// Array destructuring
const [first, second = "default"] = items;
```

### Nullish Coalescing and Optional Chaining

Use `??` and `?.` for safe access and defaults.

```typescript
// Nullish coalescing for defaults (only null/undefined)
const timeout = config.timeout ?? 5000;

// Optional chaining for safe access
const userName = user?.profile?.name;
const firstItem = items?.[0];
const result = callback?.();

// Combined
const displayName = user?.profile?.name ?? "Anonymous";
```

### Object Shorthand and Computed Properties

```typescript
// Property shorthand
const name = "John";
const age = 30;
const user = { name, age };  // { name: "John", age: 30 }

// Method shorthand
const obj = {
  getValue() { return this.value; },  // Instead of getValue: function() {}
};

// Computed properties
const key = "dynamicKey";
const obj = { [key]: value, [`${prefix}_id`]: id };
```

---

## Error Handling

Custom error hierarchy with `BaseError`:

```typescript
export class HttpRequestError extends BaseError {
  override name = "HttpRequestError";
  constructor(args: { cause?: Error; url: string }) {
    super("HTTP request failed.", { cause: args.cause });
  }
}

// Usage with instanceof
try { /* ... */ } catch (err) {
  if (err instanceof HttpRequestError) throw err;
  throw new HttpRequestError({ cause: err as Error, url });
}
```

---

## Classes

- **Private fields** use `#` syntax
- **Static factory methods** for construction

```typescript
export class Secp256k1 implements KeyPair {
  #privateKey: Uint8Array;

  static fromMnemonic(mnemonic: string): Secp256k1 {
    const seed = mnemonicToSeedSync(mnemonic);
    return new Secp256k1(deriveKey(seed));
  }

  constructor(privateKey: Uint8Array) {
    this.#privateKey = privateKey;
  }
}
```

---

## Import Organization

Order: external → internal → types

```typescript
// External
import { secp256k1 } from "@noble/curves/secp256k1";

// Internal
import { encodeHex } from "../../encoding/hex.js";

// Types
import type { Hex } from "../../types/index.js";
```

---

## React Components

### Functional Components

```typescript
interface HeaderProps {
  isScrolled: boolean;
}

export const Header: React.FC<HeaderProps> = ({ isScrolled }) => {
  return <header className={twMerge("fixed", isScrolled && "shadow-lg")} />;
};
```

### Polymorphic Components

```typescript
export const Button = forwardRefPolymorphic<"button", ButtonProps>(
  ({ as, variant, className, children, ...props }, ref) => {
    const Component = as ?? "button";
    return <Component className={twMerge(buttonVariants({ variant }), className)} ref={ref} {...props}>{children}</Component>;
  },
);

// Usage: <Button as={Link} to="/home">Home</Button>
```

### Component Composition

```typescript
const BridgeContainer: React.FC<PropsWithChildren> = ({ children }) => <div>{children}</div>;
const BridgeDeposit: React.FC = () => { /* ... */ };

export const Bridge = Object.assign(BridgeContainer, {
  Deposit: BridgeDeposit,
});

// Usage: <Bridge><Bridge.Deposit /></Bridge>
```

---

## State Management

### Store Hooks

```typescript
const { account, isConnected } = useAccount();
const { data: balances = {} } = useBalances({ address: account?.address });
```

### Context Pattern

```typescript
const [SigninProvider, useSignin] = createContext<ReturnType<typeof useSigninState>>({
  name: "SigninContext",
});

export const Signin: React.FC = () => {
  const state = useSigninState();
  return (
    <SigninProvider value={state}>
      <Content /> {/* Can call useSignin() */}
    </SigninProvider>
  );
};
```

---

## Custom Hooks

| Hook | Purpose |
|------|---------|
| `useInputs` | Form state with validation/masking |
| `useControlledState` | Dual controlled/uncontrolled state |
| `useAsyncFn` | Async operation state |
| `useMediaQuery` | Responsive breakpoints |
| `useClickAway` | Click outside detection |
| `useMountedState` | Prevent updates after unmount |

```typescript
// useInputs example
const { register, handleSubmit } = useInputs({ initialValues: { email: "" } });
const emailField = register("email", {
  validate: (v) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v) || "Invalid email",
});

// useControlledState - supports both modes
const [value, setValue] = useControlledState(propValue, onChange, defaultValue);
```

---

## Styling

### tailwind-variants

```typescript
const buttonVariants = tv({
  base: "inline-flex items-center justify-center transition-all",
  variants: {
    variant: {
      primary: "bg-red-400 hover:bg-red-600",
      secondary: "bg-blue-400 hover:bg-blue-600",
    },
    size: {
      sm: "h-8 px-2",
      md: "h-10 px-3",
    },
    isDisabled: { true: "pointer-events-none opacity-50" },
  },
  defaultVariants: { size: "md", variant: "primary" },
  compoundVariants: [
    { variant: "primary", isDisabled: true, class: "bg-gray-300" },
  ],
}, { twMerge: true });
```

### Slot-based Variants

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

### twMerge for Composition

```typescript
className={twMerge(
  "fixed bottom-0 lg:top-0",
  isScrolled && "shadow-lg",
  { "bg-white": isActive },
)}
```

---

## Form Handling

```typescript
const { register, handleSubmit, setValue } = useInputs({
  initialValues: { email: "" },
});

const emailField = register("email", {
  strategy: "onBlur",  // "onChange" | "onSubmit" | "onBlur"
  validate: (v) => isValidEmail(v) || "Invalid email",
  mask: (v) => v.toLowerCase(),
});

<form onSubmit={handleSubmit(({ email }) => submit(email))}>
  <Input {...emailField} />
</form>
```

---

## Event Handlers

- **Props**: `on` prefix (`onChange`, `onSubmit`, `onSuccess`)
- **Inline composition**: array syntax for multiple actions

```typescript
onClick={() => [navigate({ to: "/bridge" }), setSidebarVisibility(false)]}
```

---

## Rendering Patterns

```typescript
// Early return for conditional screens
if (screen !== "email") return null;

// Ternary for simple conditions
{isVisible ? <Component /> : null}

// List rendering
{items.map((item) => <Card key={item.id} {...item} />)}
```

---

## Animation (Framer Motion)

```typescript
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

// Layout animations
<motion.div layout>{content}</motion.div>

// Polymorphic
<Button as={motion.div} isLoading={isPending}>{label}</Button>
```

---

## Routing (TanStack Router)

```typescript
// Navigation
const navigate = useNavigate();
navigate({ to: "/bridge" });

// Link
<Link to="/">Home</Link>
<Button as={Link} to="/bridge">Bridge</Button>

// Router state
const { location } = useRouterState();
const { history } = useRouter();
history.go(-1);
```

---

## Internationalization (Paraglide)

```typescript
import { m } from "@left-curve/foundation/paraglide/messages.js";

<h1>{m["common.signin"]()}</h1>
<span>{m["bridge.network"]({ network })}</span>
```

---

## Quick Reference

| Category | Key Pattern |
|----------|-------------|
| Exports | Named only, barrel files, `.js` extensions |
| Types | Interfaces for contracts, types for unions |
| Code Style | `const` over `let`, IIFE for scope, immutable ops, early returns |
| Classes | `#` private fields, static factories |
| Components | `React.FC`, polymorphic, `Object.assign` composition |
| State | Store hooks, `createContext` utility |
| Styling | `tailwind-variants`, `twMerge` |
| Forms | `useInputs` with validation strategies |
| Animation | Framer Motion with `AnimatePresence` |
| Routing | TanStack Router, `useNavigate` |
| I18N | Paraglide `m["key"]()` |
