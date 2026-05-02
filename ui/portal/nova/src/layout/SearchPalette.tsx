import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Pressable, Text, TextInput, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useNavigate } from "@tanstack/react-router";
import { useSearch, openSearch, closeSearch } from "./useSearch";

// ---------- data types ----------

type SearchTarget = {
  readonly route: string;
};

type SearchItem = {
  readonly id: string;
  readonly label: string;
  readonly sub: string;
  readonly icon: string;
  readonly target?: SearchTarget;
  readonly change?: number;
};

type SearchGroup = {
  readonly title: string;
  readonly items: readonly SearchItem[];
};

// ---------- static datasets ----------

const APPS: readonly SearchItem[] = [
  {
    id: "trade",
    label: "Pro Trade",
    sub: "Perpetuals, spot, options",
    icon: "\u25B6",
    target: { route: "/trade" },
  },
  {
    id: "margin",
    label: "Margin",
    sub: "Cross & isolated positions",
    icon: "\u25A0",
    target: { route: "/trade" },
  },
  {
    id: "earn",
    label: "Earn",
    sub: "Vaults & yield strategies",
    icon: "\u2605",
    target: { route: "/earn" },
  },
  {
    id: "liquidity",
    label: "Liquidity",
    sub: "Provide & manage LP",
    icon: "\u25C6",
    target: { route: "/earn" },
  },
  {
    id: "explorer",
    label: "Block Explorer",
    sub: "Transactions, blocks, addrs",
    icon: "\u2315",
    target: { route: "/explorer" },
  },
  {
    id: "multisig",
    label: "Multisig",
    sub: "Shared wallets",
    icon: "\u2616",
    target: { route: "/account" },
  },
];

const MARKETS: readonly SearchItem[] = [
  {
    id: "btc",
    label: "BTC-PERP",
    sub: "Bitcoin perpetual",
    icon: "\u25B6",
    change: 1.42,
    target: { route: "/trade" },
  },
  {
    id: "eth",
    label: "ETH-PERP",
    sub: "Ethereum perpetual",
    icon: "\u25B6",
    change: -0.84,
    target: { route: "/trade" },
  },
  {
    id: "sol",
    label: "SOL-PERP",
    sub: "Solana perpetual",
    icon: "\u25B6",
    change: 3.18,
    target: { route: "/trade" },
  },
  {
    id: "arb",
    label: "ARB-PERP",
    sub: "Arbitrum perpetual",
    icon: "\u25B6",
    change: -1.12,
    target: { route: "/trade" },
  },
];

const SECTIONS: readonly SearchItem[] = [
  {
    id: "overview",
    label: "Overview",
    sub: "Equity, balances, activity",
    icon: "\u25A3",
    target: { route: "/account/overview" },
  },
  {
    id: "portfolio",
    label: "Portfolio",
    sub: "Holdings & allocation",
    icon: "\u25A1",
    target: { route: "/account/portfolio" },
  },
  {
    id: "history",
    label: "Trade History",
    sub: "Past fills & funding",
    icon: "\u29D6",
    target: { route: "/trade" },
  },
  {
    id: "transfers",
    label: "Transfers",
    sub: "Deposits & withdrawals",
    icon: "\u21C4",
    target: { route: "/move" },
  },
  {
    id: "preferences",
    label: "Preferences",
    sub: "Settings & configuration",
    icon: "\u2699",
    target: { route: "/account/preferences" },
  },
  {
    id: "security",
    label: "Security",
    sub: "Keys & permissions",
    icon: "\u26BF",
    target: { route: "/account/security" },
  },
];

const RECENTS: readonly SearchItem[] = [
  {
    id: "r1",
    label: "BTC-PERP",
    sub: "Recent market",
    icon: "\u29D6",
    target: { route: "/trade" },
  },
  {
    id: "r2",
    label: "Portfolio",
    sub: "Recent section",
    icon: "\u29D6",
    target: { route: "/account/portfolio" },
  },
  {
    id: "r3",
    label: "Overview",
    sub: "Recent section",
    icon: "\u29D6",
    target: { route: "/account/overview" },
  },
];

// ---------- filter helper ----------

function filterItems(items: readonly SearchItem[], query: string): readonly SearchItem[] {
  const lower = query.toLowerCase();
  return items.filter(
    (item) => item.label.toLowerCase().includes(lower) || item.sub.toLowerCase().includes(lower),
  );
}

// ---------- Kbd (keyboard shortcut badge) ----------

function Kbd({ children, className }: { children: string; className?: string }) {
  return (
    <View
      className={twMerge(
        "min-w-[18px] h-[18px] px-1",
        "items-center justify-center",
        "rounded-[4px]",
        "bg-bg-sunk border border-border-subtle",
        className,
      )}
    >
      <Text className="text-fg-tertiary text-[11px] font-mono font-medium leading-none">
        {children}
      </Text>
    </View>
  );
}

// ---------- SearchOverlay (command palette modal) ----------

function SearchOverlay() {
  const { open } = useSearch();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<TextInput>(null);
  const navigate = useNavigate();

  // Reset state when opening
  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Build grouped results
  const groups: readonly SearchGroup[] = useMemo(() => {
    if (query.trim() === "") {
      return [
        { title: "Recent", items: RECENTS },
        { title: "Apps", items: APPS.slice(0, 4) },
        { title: "Markets", items: MARKETS.slice(0, 3) },
      ];
    }
    return [
      { title: "Apps", items: filterItems(APPS, query) },
      { title: "Markets", items: filterItems(MARKETS, query) },
      { title: "Sections", items: filterItems(SECTIONS, query) },
    ].filter((g) => g.items.length > 0);
  }, [query]);

  const flatItems = useMemo(() => groups.flatMap((g) => g.items), [groups]);

  // Reset active index on query change
  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  const handleSelect = useCallback(
    (item: SearchItem) => {
      if (item.target) {
        navigate({ to: item.target.route });
      }
      closeSearch();
    },
    [navigate],
  );

  // Keyboard navigation inside the overlay
  useEffect(() => {
    if (!open) return;

    const handler = (e: KeyboardEvent) => {
      switch (e.key) {
        case "Escape":
          closeSearch();
          break;
        case "ArrowDown":
          e.preventDefault();
          setActiveIndex((prev) => Math.min(prev + 1, flatItems.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setActiveIndex((prev) => Math.max(prev - 1, 0));
          break;
        case "Enter": {
          e.preventDefault();
          const selected = flatItems[activeIndex];
          if (selected) handleSelect(selected);
          break;
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, flatItems, activeIndex, handleSelect]);

  if (!open) return null;

  let itemCounter = 0;

  return (
    <View className="fixed inset-0 z-50 items-center justify-start pt-[12vh]">
      {/* Backdrop */}
      <Pressable
        className="absolute inset-0 bg-black/35 backdrop-blur-[2px]"
        onPress={closeSearch}
        accessibilityLabel="Close search"
      />

      {/* Card */}
      <View
        className={twMerge(
          "w-[640px] max-w-[calc(100vw-32px)] z-10",
          "max-h-[70vh]",
          "bg-bg-elev",
          "border border-border-default",
          "rounded-card",
          "shadow-xl",
          "overflow-hidden",
          "flex flex-col",
        )}
      >
        {/* Input bar */}
        <View
          className={twMerge(
            "flex-row items-center gap-3 px-4 h-[52px]",
            "border-b border-border-subtle",
          )}
        >
          <Text className="text-fg-tertiary text-[18px]">{"\u2315"}</Text>
          <TextInput
            ref={inputRef}
            value={query}
            onChangeText={setQuery}
            placeholder="Search transactions, address or apps"
            placeholderTextColor="var(--fg-tertiary)"
            className={twMerge(
              "flex-1 h-full bg-transparent outline-none border-0",
              "text-fg-primary",
              "text-[15px]",
              "font-text",
            )}
            autoComplete="off"
            spellCheck={false}
          />
          <View
            className={twMerge(
              "px-1.5 py-0.5",
              "rounded-[4px]",
              "bg-bg-sunk",
              "border border-border-subtle",
            )}
          >
            <Text className="text-fg-tertiary text-[11px] font-mono font-medium">Esc</Text>
          </View>
        </View>

        {/* Results */}
        <View className="flex-1 overflow-y-auto py-1.5 px-1.5">
          {flatItems.length === 0 && (
            <View className="items-center py-9 gap-1">
              <Text className="text-fg-secondary font-medium text-[13px]">No results</Text>
              <Text className="text-fg-tertiary text-[11px]">
                Try a market symbol, an app name, or a transaction hash.
              </Text>
            </View>
          )}

          {groups.map((group) => (
            <View key={group.title} className="py-0.5">
              <View className="mt-1.5">
                <Text
                  className={twMerge(
                    "px-3 py-1",
                    "text-fg-tertiary",
                    "text-[10px]",
                    "font-semibold",
                    "tracking-caps",
                    "uppercase",
                  )}
                >
                  {group.title}
                </Text>
              </View>
              {group.items.map((item) => {
                const idx = itemCounter++;
                const isActive = idx === activeIndex;
                return (
                  <Pressable
                    key={item.id}
                    onPress={() => handleSelect(item)}
                    onHoverIn={() => setActiveIndex(idx)}
                    className={twMerge(
                      "flex-row items-center gap-3 px-3 h-9 rounded-field",
                      "transition-[background] duration-100 ease-[var(--ease)]",
                      isActive ? "bg-bg-tint" : "bg-transparent",
                    )}
                    accessibilityRole="button"
                    aria-selected={isActive}
                  >
                    {/* Icon */}
                    <View
                      className={twMerge(
                        "w-[22px] h-[22px] items-center justify-center",
                        "rounded-[6px] shrink-0",
                        isActive ? "bg-accent-bg" : "bg-bg-sunk",
                      )}
                    >
                      <Text
                        className={twMerge(
                          "text-[11px]",
                          isActive ? "text-fg-primary" : "text-fg-secondary",
                        )}
                      >
                        {item.icon}
                      </Text>
                    </View>

                    {/* Label */}
                    <Text className={twMerge("text-fg-primary", "text-[13px]", "font-medium")}>
                      {item.label}
                    </Text>

                    {/* Sub / Change */}
                    {item.sub && (
                      <Text
                        className={twMerge(
                          "ml-auto text-[12px] tabular-nums",
                          item.change !== undefined
                            ? item.change >= 0
                              ? "text-up"
                              : "text-down"
                            : "text-fg-tertiary",
                        )}
                      >
                        {item.sub}
                      </Text>
                    )}

                    {/* Enter hint */}
                    {isActive && (
                      <Text className="text-fg-tertiary text-[12px] ml-2">{"\u21B5"}</Text>
                    )}
                  </Pressable>
                );
              })}
            </View>
          ))}
        </View>

        {/* Footer hints */}
        <View
          className={twMerge(
            "flex-row items-center gap-[18px] px-3.5 py-2",
            "border-t border-border-subtle",
          )}
        >
          <View className="flex-row items-center gap-1">
            <Kbd>{"\u2191"}</Kbd>
            <Kbd>{"\u2193"}</Kbd>
            <Text className="text-fg-tertiary text-[10px] ml-0.5">navigate</Text>
          </View>
          <View className="flex-row items-center gap-1">
            <Kbd>{"\u21B5"}</Kbd>
            <Text className="text-fg-tertiary text-[10px] ml-0.5">open</Text>
          </View>
          <View className="flex-row items-center gap-1">
            <Kbd>Esc</Kbd>
            <Text className="text-fg-tertiary text-[10px] ml-0.5">close</Text>
          </View>
        </View>
      </View>
    </View>
  );
}

// ---------- CompactTrigger (header search button) ----------

function CompactTrigger({ className }: { className?: string }) {
  return (
    <Pressable
      onPress={openSearch}
      accessibilityLabel="Open search"
      className={twMerge(
        "inline-flex flex-row items-center gap-2",
        "h-8 pl-2.5 pr-2",
        "rounded-field",
        "bg-bg-surface border border-border-default",
        "text-fg-tertiary text-[12px]",
        "transition-[background,border-color,color] duration-150 ease-[var(--ease)]",
        "hover:bg-bg-tint hover:border-border-strong hover:text-fg-secondary",
        "cursor-pointer",
        className,
      )}
    >
      <Text className="text-fg-tertiary text-[13px]">{"\u2315"}</Text>
      <Text className="text-fg-tertiary text-[12px] font-text flex-1">Search...</Text>
      <View className="flex-row items-center gap-0.5 shrink-0">
        <Kbd>{"\u2318"}</Kbd>
        <Kbd>K</Kbd>
      </View>
    </Pressable>
  );
}

// ---------- HeroTrigger (Account page large search bar) ----------

function HeroTrigger({ className }: { className?: string }) {
  return (
    <Pressable
      onPress={openSearch}
      accessibilityLabel="Open search"
      className={twMerge(
        "flex flex-row items-center gap-3",
        "w-full h-14 px-4",
        "bg-bg-surface border border-border-default rounded-card",
        "transition-[border-color,background,box-shadow] duration-150 ease-[var(--ease)]",
        "hover:border-border-strong hover:bg-bg-elev",
        "cursor-pointer",
        className,
      )}
    >
      <Text className="text-fg-tertiary text-[20px]">{"\u2315"}</Text>
      <Text className="text-fg-secondary text-[14px] font-display">
        Search transactions, address
      </Text>
      <View
        className={twMerge(
          "ml-auto flex-row items-center gap-1",
          "pl-4",
          "border-l border-border-subtle",
        )}
      >
        <Text className="text-fg-tertiary text-[13px] mr-2">or apps</Text>
        <View className="flex-row items-center gap-0.5">
          <Kbd>{"\u2318"}</Kbd>
          <Kbd>K</Kbd>
        </View>
      </View>
    </Pressable>
  );
}

// ---------- Compound export ----------

export const SearchPalette = Object.assign(SearchOverlay, {
  Compact: CompactTrigger,
  Hero: HeroTrigger,
});
