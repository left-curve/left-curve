import { useState } from "react";
import { View, Text, ScrollView } from "react-native";

import { Button } from "./Button";
import { IconButton } from "./IconButton";
import { Badge } from "./Badge";
import { Chip } from "./Chip";
import { Card } from "./Card";
import { Input } from "./Input";
import { Tabs } from "./Tabs";
import { Toggle } from "./Toggle";
import { Spinner } from "./Spinner";
import { Skeleton } from "./Skeleton";
import { Dot } from "./Dot";
import { Divider } from "./Divider";

/* ---------- Layout primitives ---------- */

function Section({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}) {
  return (
    <View className="flex flex-col gap-5">
      <View>
        <Text className="text-[11px] font-semibold tracking-widest uppercase text-fg-tertiary">
          Component
        </Text>
        <Text
          className="text-[28px] font-semibold tracking-tight text-fg-primary mt-1.5"
          style={{ fontFamily: "var(--font-display)" }}
        >
          {title}
        </Text>
        {subtitle && (
          <Text
            className="text-[14px] text-fg-secondary mt-1 leading-relaxed"
            style={{ maxWidth: 720 }}
          >
            {subtitle}
          </Text>
        )}
      </View>
      {children}
    </View>
  );
}

function Specimen({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <View className="flex flex-col gap-2.5">
      <View className="flex flex-row items-baseline justify-between gap-3 flex-wrap">
        <Text className="text-[12px] font-medium text-fg-primary whitespace-nowrap">{label}</Text>
        {hint && (
          <Text className="text-[11px] text-fg-tertiary" style={{ fontFamily: "var(--font-mono)" }}>
            {hint}
          </Text>
        )}
      </View>
      <View className="p-6 bg-bg-surface border border-border-subtle rounded-card flex flex-row flex-wrap gap-3 items-center">
        {children}
      </View>
    </View>
  );
}

function Subhead({ children, hint }: { children: string; hint?: string }) {
  return (
    <View className="flex flex-row items-baseline gap-2 pb-1 border-b border-border-subtle">
      <Text className="text-[13px] font-semibold tracking-tight whitespace-nowrap text-fg-primary">
        {children}
      </Text>
      {hint && <Text className="text-[11px] text-fg-tertiary">{hint}</Text>}
    </View>
  );
}

/* ---------- Sections ---------- */

const BUTTON_VARIANTS = ["primary", "secondary", "ghost", "up", "down"] as const;
const BUTTON_SIZES = ["sm", "default", "lg"] as const;

function ButtonsSection() {
  return (
    <Section
      title="Buttons"
      subtitle="Compact, hairline-bordered buttons. Color is reserved for primary CTAs and trade actions; secondary and ghost carry the rest."
    >
      <Subhead hint="primary, secondary, ghost, up, down">Variants</Subhead>
      <Specimen label="Variant matrix" hint="default size">
        {BUTTON_VARIANTS.map((v) => (
          <Button key={v} variant={v}>
            <Text>{v.charAt(0).toUpperCase() + v.slice(1)}</Text>
          </Button>
        ))}
      </Specimen>

      <Subhead hint="sm 28, default 32, lg 44">Sizes</Subhead>
      <Specimen label="Size scale -- primary">
        {BUTTON_SIZES.map((s) => (
          <Button key={s} variant="primary" size={s}>
            <Text>{s === "default" ? "Medium" : s === "sm" ? "Small" : "Large"}</Text>
          </Button>
        ))}
      </Specimen>
      <Specimen label="Size scale -- secondary">
        {BUTTON_SIZES.map((s) => (
          <Button key={s} variant="secondary" size={s}>
            <Text>{s === "default" ? "Medium" : s === "sm" ? "Small" : "Large"}</Text>
          </Button>
        ))}
      </Specimen>

      <Subhead>All variants x all sizes</Subhead>
      {BUTTON_VARIANTS.map((v) => (
        <Specimen key={v} label={v} hint={`variant="${v}"`}>
          {BUTTON_SIZES.map((s) => (
            <Button key={s} variant={v} size={s}>
              <Text>{s === "default" ? "Medium" : s === "sm" ? "Small" : "Large"}</Text>
            </Button>
          ))}
        </Specimen>
      ))}

      <Subhead>Disabled states</Subhead>
      <Specimen label="Disabled -- all variants">
        {BUTTON_VARIANTS.map((v) => (
          <Button key={v} variant={v} disabled>
            <Text>{v.charAt(0).toUpperCase() + v.slice(1)}</Text>
          </Button>
        ))}
      </Specimen>

      <Subhead>Icon-only buttons</Subhead>
      <Specimen label="iconOnly prop" hint="square shape, no text">
        {BUTTON_VARIANTS.map((v) => (
          <Button key={v} variant={v} iconOnly>
            <Text style={{ fontSize: 14 }}>+</Text>
          </Button>
        ))}
      </Specimen>

      <Subhead>Full width</Subhead>
      <Specimen label="Block / form submission">
        <View className="w-full">
          <Button variant="primary" size="lg" className="w-full">
            <Text>Open long -- BTC-PERP</Text>
          </Button>
        </View>
      </Specimen>

      <Subhead>With spinner</Subhead>
      <Specimen label="Loading state">
        <Button variant="primary">
          <Spinner size="sm" />
          <Text>Submitting</Text>
        </Button>
        <Button variant="secondary">
          <Spinner size="sm" />
          <Text>Loading</Text>
        </Button>
      </Specimen>
    </Section>
  );
}

const BADGE_VARIANTS = ["default", "up", "down", "warn", "accent", "outline"] as const;

function BadgesSection() {
  return (
    <Section
      title="Badge"
      subtitle="Label small, immutable facts. Badges are status indicators -- filled, on, alerts."
    >
      <Subhead hint="default, up, down, warn, accent, outline">Variants</Subhead>
      <Specimen label="All variants">
        {BADGE_VARIANTS.map((v) => (
          <Badge key={v} variant={v}>
            <Text>{v.charAt(0).toUpperCase() + v.slice(1)}</Text>
          </Badge>
        ))}
      </Specimen>

      <Subhead>Status use cases</Subhead>
      <Specimen label="Contextual labels">
        <Badge variant="up">
          <Text>Filled</Text>
        </Badge>
        <Badge variant="down">
          <Text>Cancelled</Text>
        </Badge>
        <Badge variant="warn">
          <Text>Expiring</Text>
        </Badge>
        <Badge variant="accent">
          <Text>Live</Text>
        </Badge>
        <Badge variant="default">
          <Text>Pending</Text>
        </Badge>
        <Badge variant="outline">
          <Text>Draft</Text>
        </Badge>
      </Specimen>

      <Subhead>Numeric / count</Subhead>
      <Specimen label="With numbers">
        <Badge variant="up">
          <Text>+ $348.30</Text>
        </Badge>
        <Badge variant="down">
          <Text>- $87.40</Text>
        </Badge>
        <Badge variant="warn">
          <Text>3 days</Text>
        </Badge>
        <Badge variant="accent" className="min-w-[18px] justify-center">
          <Text>3</Text>
        </Badge>
        <View className="flex flex-row items-center gap-1">
          <Text className="text-[12px] text-fg-secondary">Notifications</Text>
          <Badge variant="down" className="min-w-[18px] justify-center px-[5px]">
            <Text>12</Text>
          </Badge>
        </View>
      </Specimen>
    </Section>
  );
}

const CHIP_VARIANTS = ["default", "up", "down", "accent", "outline"] as const;

function ChipsSection() {
  return (
    <Section
      title="Chip"
      subtitle="Filterable, removable, interactive labels. Used for network tags, token identifiers, and active filters."
    >
      <Subhead hint="default, up, down, accent, outline">Variants</Subhead>
      <Specimen label="All variants">
        {CHIP_VARIANTS.map((v) => (
          <Chip key={v} variant={v}>
            <Text>{v.charAt(0).toUpperCase() + v.slice(1)}</Text>
          </Chip>
        ))}
      </Specimen>

      <Subhead>Network / tag use cases</Subhead>
      <Specimen label="Contextual labels">
        <Chip variant="outline">
          <Text>Ethereum</Text>
        </Chip>
        <Chip variant="outline">
          <Text>Arbitrum</Text>
        </Chip>
        <Chip variant="outline">
          <Text>Solana</Text>
        </Chip>
        <Chip variant="default">
          <Text>USD</Text>
        </Chip>
        <Chip variant="default">
          <Text>Cross 10x</Text>
        </Chip>
      </Specimen>

      <Subhead>With leading dot</Subhead>
      <Specimen label="Status pair">
        <Chip variant="outline">
          <Dot variant="up" />
          <Text>Synced</Text>
        </Chip>
        <Chip variant="outline">
          <Dot variant="warn" />
          <Text>Pending</Text>
        </Chip>
        <Chip variant="outline">
          <Dot variant="down" />
          <Text>Offline</Text>
        </Chip>
      </Specimen>
    </Section>
  );
}

function CardsSection() {
  return (
    <Section
      title="Card"
      subtitle="One base shape: hairline border, soft single shadow, 10px radius. Cards never stack shadows; elevation is conveyed by background lift."
    >
      <Subhead hint="default, elevated, sunken">Variants</Subhead>
      <View className="flex flex-row flex-wrap gap-4">
        <View className="flex-1 min-w-[280px]">
          <Specimen label="Default -- card" hint="bg-surface">
            <Card className="w-full p-[18px]">
              <Text className="text-[11px] font-semibold tracking-widest uppercase text-fg-tertiary">
                Available margin
              </Text>
              <Text className="text-[28px] font-medium tabular-nums mt-1.5 tracking-tight text-fg-primary">
                $12,482.04
              </Text>
              <Text className="text-[12px] text-fg-tertiary mt-1.5">Across 2 chains</Text>
            </Card>
          </Specimen>
        </View>
        <View className="flex-1 min-w-[280px]">
          <Specimen label="Elevated -- for menus / modals" hint="bg-elev + shadow">
            <Card variant="elevated" className="w-full p-[18px]">
              <Text className="text-[11px] font-semibold tracking-widest uppercase text-fg-tertiary">
                Floating
              </Text>
              <Text className="text-[14px] font-medium mt-1.5 text-fg-primary">
                Popover-style content
              </Text>
              <Text className="text-[12px] text-fg-tertiary mt-1">
                Slightly lifted to read above the canvas
              </Text>
            </Card>
          </Specimen>
        </View>
        <View className="flex-1 min-w-[280px]">
          <Specimen label="Sunken -- dense data groups" hint="bg-sunk, no shadow">
            <Card variant="sunken" className="w-full p-[18px]">
              <Text className="text-[11px] font-semibold tracking-widest uppercase text-fg-tertiary">
                Order summary
              </Text>
              <Text className="text-[14px] font-medium mt-1.5 text-fg-primary">
                Nested content area
              </Text>
              <Text className="text-[12px] text-fg-tertiary mt-1">
                Recessed below the main surface
              </Text>
            </Card>
          </Specimen>
        </View>
      </View>

      <Subhead>Card with header</Subhead>
      <Specimen label="Structured card">
        <Card className="w-full">
          <View className="flex flex-row items-center justify-between px-4 py-3 border-b border-border-subtle">
            <Text className="text-[13px] font-semibold text-fg-primary">Holdings</Text>
            <Button variant="ghost" size="sm">
              <Text>View all</Text>
            </Button>
          </View>
          <View className="p-4">
            <Text className="text-[12px] text-fg-secondary">
              Body content with consistent 16px padding.
            </Text>
          </View>
        </Card>
      </Specimen>

      <Subhead>KPI tile</Subhead>
      <Specimen label="Data display card">
        <Card className="w-full p-[18px]">
          <Text className="text-[11px] font-semibold tracking-widest uppercase text-fg-tertiary">
            Unrealized PnL
          </Text>
          <Text
            className="text-[28px] font-medium tabular-nums mt-1.5 tracking-tight"
            style={{ color: "var(--color-up)" }}
          >
            +$348.30
          </Text>
          <Text className="text-[12px] text-fg-tertiary mt-1.5">2 open positions</Text>
        </Card>
      </Specimen>

      <Subhead>Empty state</Subhead>
      <Specimen label="No data placeholder">
        <Card className="w-full p-8 items-center">
          <View
            className="w-9 h-9 rounded-lg items-center justify-center"
            style={{ backgroundColor: "var(--bg-tint)" }}
          >
            <Text className="text-fg-tertiary text-[18px]">L</Text>
          </View>
          <Text className="text-[13px] font-medium mt-2.5 text-fg-primary">No open positions</Text>
          <Text className="text-[12px] text-fg-tertiary mt-1">Open one from the trade form.</Text>
        </Card>
      </Specimen>
    </Section>
  );
}

function InputsSection() {
  return (
    <Section
      title="Input"
      subtitle="One field shape, multiple roles. Numeric inputs use tabular numerics; stacked variant is reserved for hero numbers."
    >
      <Subhead>Default</Subhead>
      <Specimen label="Plain input">
        <View className="w-full max-w-xs">
          <Input placeholder="Enter address" />
        </View>
      </Specimen>

      <Subhead>With label</Subhead>
      <Specimen label="Labeled input">
        <View className="w-full max-w-xs">
          <Input label="Price (USD)" placeholder="0.00" />
        </View>
      </Specimen>

      <Subhead>With error</Subhead>
      <Specimen label="Validation error">
        <View className="w-full max-w-xs">
          <Input label="Amount" defaultValue="0.000" error="Insufficient balance" />
        </View>
      </Specimen>

      <Subhead>Disabled</Subhead>
      <Specimen label="Locked field">
        <View className="w-full max-w-xs">
          <Input label="Destination" defaultValue="Locked value" disabled />
        </View>
      </Specimen>

      <Subhead>With prefix</Subhead>
      <Specimen label="Leading icon / text">
        <View className="w-full max-w-xs">
          <Input
            placeholder="Search markets"
            prefix={<Text className="text-fg-tertiary text-[12px]">S</Text>}
          />
        </View>
      </Specimen>

      <Subhead>With suffix</Subhead>
      <Specimen label="Trailing unit">
        <View className="w-full max-w-xs">
          <Input
            defaultValue="67482.40"
            suffix={<Text className="text-fg-tertiary text-[12px]">USD</Text>}
          />
        </View>
      </Specimen>

      <Subhead>With prefix and suffix</Subhead>
      <Specimen label="Both sides">
        <View className="w-full max-w-xs">
          <Input
            label="Size"
            defaultValue="0.420"
            prefix={<Text className="text-fg-tertiary text-[12px]">BTC</Text>}
            suffix={<Text className="text-fg-tertiary text-[12px]">~$28,342</Text>}
          />
        </View>
      </Specimen>

      <Subhead>All states side by side</Subhead>
      <View className="flex flex-row flex-wrap gap-4">
        <View className="flex-1 min-w-[220px]">
          <Specimen label="Normal">
            <View className="w-full">
              <Input placeholder="Normal state" />
            </View>
          </Specimen>
        </View>
        <View className="flex-1 min-w-[220px]">
          <Specimen label="Error">
            <View className="w-full">
              <Input defaultValue="Invalid" error="Required field" />
            </View>
          </Specimen>
        </View>
        <View className="flex-1 min-w-[220px]">
          <Specimen label="Disabled">
            <View className="w-full">
              <Input defaultValue="Disabled" disabled />
            </View>
          </Specimen>
        </View>
      </View>
    </Section>
  );
}

const TAB_ITEMS = [
  { value: "all", label: "All" },
  { value: "spot", label: "Spot" },
  { value: "perp", label: "Perp" },
  { value: "earn", label: "Earn" },
] as const;

const LEVERAGE_ITEMS = [
  { value: "1x", label: "1x" },
  { value: "5x", label: "5x" },
  { value: "10x", label: "10x" },
  { value: "25x", label: "25x" },
  { value: "50x", label: "50x" },
  { value: "100x", label: "100x" },
] as const;

const SECTION_ITEMS = [
  { value: "positions", label: "Positions" },
  { value: "orders", label: "Open orders" },
  { value: "fills", label: "Fills" },
  { value: "funding", label: "Funding" },
  { value: "history", label: "History" },
] as const;

function TabsSection() {
  return (
    <Section
      title="Tabs"
      subtitle="Two tab styles: segmented (pill, for card headers) and underline (bottom border, for section navigation). Both support controlled and uncontrolled modes."
    >
      <Subhead hint="pill / segmented style">Segmented (default)</Subhead>
      <Specimen label="Inside a card header">
        <Tabs items={[...TAB_ITEMS]} defaultValue="all" />
      </Specimen>

      <Subhead>Leverage picker</Subhead>
      <Specimen label="Position quick-pick">
        <Tabs items={[...LEVERAGE_ITEMS]} defaultValue="10x" />
      </Specimen>

      <Subhead hint="bottom-border active indicator">Underline variant</Subhead>
      <Specimen label="Section navigation">
        <View className="w-full">
          <Tabs variant="underline" items={[...SECTION_ITEMS]} defaultValue="positions" />
        </View>
      </Specimen>

      <Specimen label="Sub-page tabs">
        <View className="w-full">
          <Tabs
            variant="underline"
            items={[
              { value: "overview", label: "Overview" },
              { value: "holdings", label: "Holdings" },
              { value: "activity", label: "Activity" },
              { value: "earn", label: "Earn" },
              { value: "settings", label: "Settings" },
            ]}
            defaultValue="overview"
          />
        </View>
      </Specimen>

      <Subhead>Segmented -- section nav</Subhead>
      <Specimen label="Multiple sections">
        <Tabs items={[...SECTION_ITEMS]} defaultValue="positions" />
      </Specimen>

      <Subhead>Two items</Subhead>
      <Specimen label="Binary choice">
        <Tabs
          items={[
            { value: "buy", label: "Buy / Long" },
            { value: "sell", label: "Sell / Short" },
          ]}
          defaultValue="buy"
        />
      </Specimen>
    </Section>
  );
}

function ToggleSection() {
  const [checked1, setChecked1] = useState(true);
  const [checked2, setChecked2] = useState(false);

  return (
    <Section
      title="Toggle"
      subtitle="Toggles are for instant settings -- post-only, reduce-only, dark mode. Flips immediately on press."
    >
      <Subhead>Interactive</Subhead>
      <Specimen label="Trade flags">
        <View className="flex flex-row items-center gap-2.5">
          <Toggle checked={checked1} onChange={setChecked1} />
          <Text className="text-[12px] text-fg-primary">Post-only</Text>
        </View>
        <View className="w-6" />
        <View className="flex flex-row items-center gap-2.5">
          <Toggle checked={checked2} onChange={setChecked2} />
          <Text className="text-[12px] text-fg-primary">Reduce-only</Text>
        </View>
      </Specimen>

      <Subhead>States</Subhead>
      <Specimen label="On / off / disabled">
        <View className="flex flex-row items-center gap-2.5">
          <Toggle defaultChecked />
          <Text className="text-[12px] text-fg-primary">On</Text>
        </View>
        <View className="flex flex-row items-center gap-2.5">
          <Toggle />
          <Text className="text-[12px] text-fg-primary">Off</Text>
        </View>
        <View className="flex flex-row items-center gap-2.5">
          <Toggle disabled />
          <Text className="text-[12px] text-fg-tertiary">Disabled (off)</Text>
        </View>
        <View className="flex flex-row items-center gap-2.5">
          <Toggle defaultChecked disabled />
          <Text className="text-[12px] text-fg-tertiary">Disabled (on)</Text>
        </View>
      </Specimen>
    </Section>
  );
}

function SpinnerSection() {
  return (
    <Section
      title="Spinner"
      subtitle="Loading indicator. Three sizes: sm (12px), default (16px), lg (24px). Inherits current text color."
    >
      <Subhead hint="sm, default, lg">Sizes</Subhead>
      <Specimen label="All sizes">
        <View className="flex flex-row items-center gap-2">
          <Spinner size="sm" />
          <Text className="text-[11px] text-fg-tertiary">sm</Text>
        </View>
        <View className="flex flex-row items-center gap-2">
          <Spinner size="default" />
          <Text className="text-[11px] text-fg-tertiary">default</Text>
        </View>
        <View className="flex flex-row items-center gap-2">
          <Spinner size="lg" />
          <Text className="text-[11px] text-fg-tertiary">lg</Text>
        </View>
      </Specimen>

      <Subhead>In context</Subhead>
      <Specimen label="Inside a button">
        <Button variant="primary">
          <Spinner size="sm" />
          <Text>Processing</Text>
        </Button>
        <Button variant="secondary">
          <Spinner size="sm" />
          <Text>Loading</Text>
        </Button>
        <Button variant="ghost">
          <Spinner size="sm" />
          <Text>Fetching</Text>
        </Button>
      </Specimen>

      <Subhead>Standalone</Subhead>
      <Specimen label="Centered loading">
        <View className="w-full flex items-center justify-center py-8">
          <Spinner size="lg" />
        </View>
      </Specimen>
    </Section>
  );
}

function SkeletonSection() {
  return (
    <Section
      title="Skeleton"
      subtitle="Pulsing placeholder for loading states. Matches the dimensions of the content it replaces."
    >
      <Subhead>Various sizes</Subhead>
      <Specimen label="Text lines">
        <View className="flex flex-col gap-2 w-full">
          <Skeleton width="60%" height={14} />
          <Skeleton width="100%" height={14} />
          <Skeleton width="80%" height={14} />
        </View>
      </Specimen>

      <Specimen label="Card placeholder">
        <View className="flex flex-col gap-3 w-full">
          <Skeleton width="40%" height={12} />
          <Skeleton width="50%" height={28} />
          <Skeleton width="30%" height={12} />
        </View>
      </Specimen>

      <Subhead>Rounded</Subhead>
      <Specimen label="Avatar / circle shapes">
        <Skeleton width={28} height={28} rounded />
        <Skeleton width={36} height={36} rounded />
        <Skeleton width={48} height={48} rounded />
      </Specimen>

      <Subhead>Table rows</Subhead>
      <Specimen label="Loading table">
        <View className="w-full flex flex-col gap-2">
          {[1, 2, 3].map((row) => (
            <View key={row} className="flex flex-row items-center gap-3">
              <Skeleton width={22} height={22} rounded />
              <Skeleton width="25%" height={14} />
              <Skeleton width="15%" height={14} />
              <View className="flex-1" />
              <Skeleton width="20%" height={14} />
            </View>
          ))}
        </View>
      </Specimen>
    </Section>
  );
}

const DOT_VARIANTS = ["default", "up", "down", "warn"] as const;
const DOT_LABELS: Record<string, string> = {
  default: "Idle",
  up: "Online",
  down: "Offline",
  warn: "Pending",
};

function DotSection() {
  return (
    <Section
      title="Dot"
      subtitle="Smallest unit of state. Used in chips, list items, header indicators -- anywhere a single byte of state needs to broadcast loudly without taking space."
    >
      <Subhead hint="default, up, down, warn">Variants</Subhead>
      <Specimen label="All variants">
        {DOT_VARIANTS.map((v) => (
          <View key={v} className="flex flex-row items-center gap-1.5">
            <Dot variant={v} />
            <Text className="text-[12px] text-fg-primary">{DOT_LABELS[v]}</Text>
          </View>
        ))}
      </Specimen>

      <Subhead>Pulsing</Subhead>
      <Specimen label="Live state animation">
        {DOT_VARIANTS.filter((v) => v !== "default").map((v) => (
          <View key={v} className="flex flex-row items-center gap-1.5">
            <Dot variant={v} pulse />
            <Text className="text-[12px] text-fg-primary">{DOT_LABELS[v]} (pulse)</Text>
          </View>
        ))}
      </Specimen>

      <Subhead>In context</Subhead>
      <Specimen label="Status indicators">
        <Card className="w-full p-3 flex flex-row items-center gap-2.5">
          <Dot variant="up" pulse />
          <Text className="text-[12px] text-fg-primary">Mainnet -- Ethereum</Text>
          <View className="flex-1" />
          <Text className="text-[11px] text-fg-tertiary">Block 18,420,392</Text>
        </Card>
      </Specimen>
    </Section>
  );
}

function DividerSection() {
  return (
    <Section
      title="Divider"
      subtitle="Horizontal rule. 1px border-subtle separator between content sections."
    >
      <Subhead>Default</Subhead>
      <Specimen label="Horizontal divider">
        <View className="w-full flex flex-col gap-4">
          <Text className="text-[12px] text-fg-secondary">Content above the divider</Text>
          <Divider />
          <Text className="text-[12px] text-fg-secondary">Content below the divider</Text>
        </View>
      </Specimen>

      <Subhead>In a card</Subhead>
      <Specimen label="Section separator">
        <Card className="w-full">
          <View className="p-4">
            <Text className="text-[12px] font-medium text-fg-primary">Section A</Text>
            <Text className="text-[12px] text-fg-tertiary mt-1">First section content</Text>
          </View>
          <Divider />
          <View className="p-4">
            <Text className="text-[12px] font-medium text-fg-primary">Section B</Text>
            <Text className="text-[12px] text-fg-tertiary mt-1">Second section content</Text>
          </View>
          <Divider />
          <View className="p-4">
            <Text className="text-[12px] font-medium text-fg-primary">Section C</Text>
            <Text className="text-[12px] text-fg-tertiary mt-1">Third section content</Text>
          </View>
        </Card>
      </Specimen>
    </Section>
  );
}

const ICON_BUTTON_SIZES = ["sm", "default", "lg"] as const;
const ICON_BUTTON_SHAPES = ["square", "circle"] as const;

function IconButtonSection() {
  return (
    <Section
      title="IconButton"
      subtitle="Ghost icon buttons for secondary actions. Two shapes (square, circle) and three sizes (sm, default, lg)."
    >
      <Subhead hint="sm, default, lg">Sizes -- square</Subhead>
      <Specimen label="Square shape">
        {ICON_BUTTON_SIZES.map((s) => (
          <View key={s} className="flex flex-row items-center gap-2">
            <IconButton size={s} shape="square">
              <Text style={{ fontSize: s === "sm" ? 12 : s === "lg" ? 18 : 14 }}>+</Text>
            </IconButton>
            <Text className="text-[11px] text-fg-tertiary">{s}</Text>
          </View>
        ))}
      </Specimen>

      <Subhead>Sizes -- circle</Subhead>
      <Specimen label="Circle shape">
        {ICON_BUTTON_SIZES.map((s) => (
          <View key={s} className="flex flex-row items-center gap-2">
            <IconButton size={s} shape="circle">
              <Text style={{ fontSize: s === "sm" ? 12 : s === "lg" ? 18 : 14 }}>X</Text>
            </IconButton>
            <Text className="text-[11px] text-fg-tertiary">{s}</Text>
          </View>
        ))}
      </Specimen>

      <Subhead>All shapes x all sizes</Subhead>
      <Specimen label="Complete matrix">
        {ICON_BUTTON_SHAPES.map((shape) =>
          ICON_BUTTON_SIZES.map((size) => (
            <View key={`${shape}-${size}`} className="flex flex-col items-center gap-1">
              <IconButton size={size} shape={shape}>
                <Text style={{ fontSize: size === "sm" ? 12 : size === "lg" ? 18 : 14 }}>C</Text>
              </IconButton>
              <Text className="text-[10px] text-fg-tertiary">
                {shape}/{size}
              </Text>
            </View>
          )),
        )}
      </Specimen>

      <Subhead>Disabled</Subhead>
      <Specimen label="Disabled icon buttons">
        <IconButton disabled shape="square">
          <Text>X</Text>
        </IconButton>
        <IconButton disabled shape="circle">
          <Text>X</Text>
        </IconButton>
      </Specimen>

      <Subhead>In context</Subhead>
      <Specimen label="Card header actions">
        <Card className="w-full flex flex-row items-center justify-between px-4 py-3">
          <Text className="text-[13px] font-semibold text-fg-primary">Holdings</Text>
          <View className="flex flex-row gap-1">
            <IconButton size="sm" shape="square">
              <Text style={{ fontSize: 12 }}>F</Text>
            </IconButton>
            <IconButton size="sm" shape="square">
              <Text style={{ fontSize: 12 }}>...</Text>
            </IconButton>
          </View>
        </Card>
      </Specimen>
    </Section>
  );
}

/* ---------- Main showcase ---------- */

export function ComponentShowcase() {
  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: "var(--bg-app)" }}
      contentContainerStyle={{
        padding: 24,
        maxWidth: 960,
        marginLeft: "auto",
        marginRight: "auto",
        width: "100%",
      }}
    >
      {/* Page header */}
      <View className="mb-8">
        <Text className="text-[11px] font-semibold tracking-widest uppercase text-fg-tertiary">
          Nova Design System
        </Text>
        <Text
          className="text-[36px] font-semibold tracking-tight text-fg-primary mt-1"
          style={{ fontFamily: "var(--font-display)" }}
        >
          Component Library
        </Text>
        <Text
          className="text-[14px] text-fg-secondary mt-2 leading-relaxed"
          style={{ maxWidth: 680 }}
        >
          Every foundation component rendered with all variants and states. Use this page to
          visually compare against the redesign reference.
        </Text>
      </View>

      <Divider className="mb-10" />

      {/* Table of contents */}
      <View className="mb-10">
        <Text className="text-[12px] font-semibold text-fg-primary mb-3">Contents</Text>
        <View className="flex flex-row flex-wrap gap-2">
          {[
            "Buttons",
            "Badge",
            "Chip",
            "Card",
            "Input",
            "Tabs",
            "Toggle",
            "Spinner",
            "Skeleton",
            "Dot",
            "Divider",
            "IconButton",
          ].map((name) => (
            <Chip key={name} variant="outline">
              <Text>{name}</Text>
            </Chip>
          ))}
        </View>
      </View>

      <Divider className="mb-10" />

      {/* All sections */}
      <View className="flex flex-col gap-16">
        <ButtonsSection />
        <Divider />
        <BadgesSection />
        <Divider />
        <ChipsSection />
        <Divider />
        <CardsSection />
        <Divider />
        <InputsSection />
        <Divider />
        <TabsSection />
        <Divider />
        <ToggleSection />
        <Divider />
        <SpinnerSection />
        <Divider />
        <SkeletonSection />
        <Divider />
        <DotSection />
        <Divider />
        <DividerSection />
        <Divider />
        <IconButtonSection />
      </View>

      {/* Footer */}
      <View className="mt-16 mb-8 items-center">
        <Text className="text-[11px] text-fg-quaternary">
          Nova UI -- Component Showcase -- All foundation components
        </Text>
      </View>
    </ScrollView>
  );
}
