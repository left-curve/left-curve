# useStorage Simplification Design

## Context

`useStorage` is a shared storage hook used across the portal web app, portal mobile app, applets kit, and foundation UI. It currently uses Zustand `persist` internally to solve shared state and async hydration, especially after the web storage backend moved to IndexedDB. The hook has become harder to reason about because it mixes React state, persistence format, migrations, hydration, and cross-runtime sync behind Zustand middleware.

The public `useStorage` API and persisted data format must remain compatible. The goal is to simplify the internal implementation while preserving current behavior for web and mobile.

## Goals

- Preserve the current `useStorage(key, options)` API.
- Preserve the current Zustand-compatible persisted payload shape.
- Remove Zustand from `useStorage` internals.
- Support async storage hydration, especially IndexedDB on web.
- Keep writes optimistic.
- Prevent stale async hydration from overwriting newer local writes.
- Make storage sync adapter-driven so web and mobile can use different mechanisms.
- Preserve cross-tab session transfer for `sessionStorage`.

## Non-Goals

- Do not change persisted keys.
- Do not migrate to a new on-disk payload format.
- Do not remove existing options such as `enabled`, `version`, `migrations`, `storage`, or `sync`.
- Do not use TanStack Query as the storage registry.
- Do not add hook-level `BroadcastChannel` logic.

## Public API

The hook keeps its current API:

```ts
useStorage<T>(key, {
  enabled,
  initialValue,
  storage,
  version,
  migrations,
  sync,
})
```

Add one optional option:

```ts
onError?: (error: unknown) => void;
```

Storage read and write errors are reported through `options.onError?.(error)`. The hook should not implicitly call `config.captureError`.

## Persisted Payload

Reads and writes target the current Zustand-compatible shape:

```ts
{
  state: {
    value,
    version
  },
  version
}
```

The implementation does not need compatibility with older pre-Zustand payloads because the current application state uses the Zustand shape.

## Architecture

Replace the Zustand store cache with a small module-local registry of entries. Each entry is scoped by storage adapter identity and logical key.

Conceptually:

```ts
WeakMap<Storage, Map<string, StorageEntry>>
```

Each `StorageEntry` owns:

- current value
- hydration state
- revision counter
- React listeners
- hydration logic
- optimistic `setValue`
- migration handling
- persistence
- adapter sync subscription

React components read entries through `useSyncExternalStore`. The registry is only the small shared state layer that Zustand currently provides for this hook.

Using `WeakMap` prevents collisions between different storage adapters using the same logical key, such as default app storage and session storage. It also avoids retaining temporary adapter objects forever. Call sites that create custom storage inline, such as session storage, should be moved to module scope or memoized so storage identity is stable.

## Data Flow

On first use, `useStorage`:

1. Resolves `storage = options.storage ?? config.storage`.
2. Computes `initialValue`.
3. Gets or creates the registry entry for `(storage, key)`.
4. Subscribes to the entry with `useSyncExternalStore`.

Before mount, the hook returns the initial value and `hasHydrated = true` to preserve current SSR and client hydration behavior.

After mount, if `enabled !== false`, the entry hydrates from storage. Hydration:

1. Reads `storage.getItem(key)`.
2. Extracts `state.value` and stored version from the persisted payload.
3. Runs migration logic if the stored version differs from the requested version.
4. Updates the entry value and sets `hasHydrated = true`.
5. Persists the migrated payload if a migration ran.

If `setValue` runs while an async read is in flight, hydration must not overwrite the newer local value. The entry tracks a revision counter and only applies hydration results when no local write happened since the read started.

## Writes

`setValue` is optimistic:

1. Resolve function updates against the current entry value.
2. Increment the entry revision.
3. Update the in-memory value.
4. Notify all entry subscribers.
5. Persist the current Zustand-compatible payload in the background.
6. Report async persistence errors through `options.onError?.(error)`.

The hook should not wait for storage before updating UI.

## Migrations

Migrations remain behavior-compatible with the current hook.

When the persisted version differs from the requested `version`:

1. If `migrations["*"]` exists, apply it to the stored value.
2. Otherwise, if `migrations[storedVersion]` exists, apply it to the stored value.
3. Otherwise, keep the stored value.

If a migration runs, persist the migrated payload at the requested version.

## Adapter-Driven Sync

Add optional value-carrying `subscribe` support to the storage interfaces.

Raw storage:

```ts
export type AbstractStorage = {
  getItem(key: string): string | null | undefined | Promise<string | null | undefined>;
  setItem(key: string, value: string): void | Promise<void>;
  removeItem(key: string): void | Promise<void>;
  subscribe?(key: string, listener: (value: string | null) => void): () => void;
};
```

Wrapped storage:

```ts
export type Storage = {
  key: string;
  getItem(...): ...;
  setItem(...): ...;
  removeItem(...): ...;
  subscribe?(key: string, listener: (value: unknown | null) => void): () => void;
};
```

`createStorage` and `createAsyncStorage` handle prefixing and deserialization:

```ts
subscribe(key, listener) {
  return storage.subscribe?.(`${prefix}.${key}`, (raw) => {
    listener(raw === null ? null : deserialize(raw));
  }) ?? (() => {});
}
```

`useStorage` only consumes `storage.subscribe?.(key, applyPersistedPayload)` when `sync: true`. It does not know whether sync is implemented with BroadcastChannel, MMKV listeners, or an in-memory listener map.

## Adapter Behavior

### IndexedDB

The IndexedDB adapter should implement `subscribe` with `BroadcastChannel`.

- `setItem` writes to IndexedDB, then broadcasts `{ key, value }`.
- `removeItem` deletes from IndexedDB, then broadcasts `{ key, value: null }`.
- `subscribe(key, listener)` listens for matching keys and passes the raw value through.

This keeps web cross-tab sync inside the adapter instead of the hook.

### MMKV

The MMKV adapter should implement `subscribe` using `storage.addOnValueChangedListener`.

When MMKV reports a changed physical key, the adapter reads the current raw string value and passes it to the listener. Removed keys pass `null`.

### Session Storage

Browser `sessionStorage` is per-tab, so a key-change notification alone cannot transfer values to another tab. The session storage adapter or wrapper should use a `BroadcastChannel` that sends the raw value.

On receiving `{ key, value }`:

- If `value` is `null`, remove the key from the receiving tab's `sessionStorage`.
- Otherwise, write the raw value into the receiving tab's `sessionStorage`.
- Notify subscribers with the raw value.

This preserves cross-tab session transfer while keeping the hook generic.

The session storage adapter should be module-scoped or otherwise stable so the registry can share state correctly.

### Memory Storage

Memory storage should keep an internal `Map<string, Set<listener>>` and call listeners with the new raw value on set/remove. This supports tests and same-process updates.

## Disabled Mode

When `enabled === false`, the hook returns:

```ts
[initialValue, setValue, false]
```

It should not hydrate or subscribe while disabled. The setter remains available, matching the current API shape.

## Error Handling

Storage read, write, migration, and subscription callback errors should be reported through `options.onError?.(error)` where practical.

The hook should not silently throw async storage failures into React render paths, and it should not implicitly report errors through config-level handlers.

## Testing

Add focused tests for:

- `createStorage` and `createAsyncStorage` prefix subscribe keys correctly.
- `createMemoryStorage` notifies subscribers on set/remove.
- `useStorage` hydrates async storage after mount and flips `hasHydrated`.
- `setValue` updates all mounted hook instances for the same storage/key.
- Async hydration cannot overwrite a newer local write.
- Migrations run against the Zustand-compatible payload and persist the migrated payload.
- `sync: true` applies value-carrying adapter subscriptions.
- `enabled: false` does not hydrate or subscribe.
- Session storage sync transfers raw values across the broadcast wrapper.

## Validation

Run:

```sh
pnpm -F @left-curve/store build
pnpm -F @left-curve/store lint
```

If tests are added under an existing Vitest setup because `@left-curve/store` has no test script yet, run the targeted Vitest command for that package or app.
