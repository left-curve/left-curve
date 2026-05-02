import { type ReactNode, useState, useCallback } from "react";
import { View, Pressable, Text } from "react-native";
import { twMerge } from "@left-curve/foundation";

export type TabItem = {
  readonly value: string;
  readonly label: ReactNode;
};

type TabVariant = "segmented" | "underline";

export type TabsProps = {
  items: readonly TabItem[];
  variant?: TabVariant;
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  className?: string;
  itemClassName?: string;
};

const containerStyles: Record<TabVariant, string> = {
  segmented: "inline-flex flex-row bg-bg-sunk border border-border-subtle rounded-btn p-0.5 gap-0",
  underline: "flex flex-row gap-1 border-b border-border-subtle",
};

const tabBaseStyles: Record<TabVariant, string> = {
  segmented: [
    "h-[26px] px-3",
    "flex-row inline-flex items-center justify-center gap-1.5",
    "border-0 bg-transparent",
    "text-fg-tertiary text-[12px] font-medium",
    "whitespace-nowrap",
    "rounded-[calc(var(--r-btn)-2px)]",
    "transition-[background,color] duration-150 ease-[var(--ease)]",
    "hover:text-fg-secondary",
  ].join(" "),
  underline: [
    "relative h-9 px-3",
    "flex-row inline-flex items-center justify-center gap-1.5",
    "border-0 bg-transparent",
    "text-fg-tertiary text-[13px] font-medium",
    "hover:text-fg-secondary",
  ].join(" "),
};

const tabActiveStyles: Record<TabVariant, string> = {
  segmented: "bg-bg-elev text-fg-primary shadow-sm",
  underline: "text-fg-primary",
};

export function Tabs({
  items,
  variant = "segmented",
  value: controlledValue,
  defaultValue,
  onChange,
  className,
  itemClassName,
}: TabsProps) {
  const [internalValue, setInternalValue] = useState(defaultValue ?? items[0]?.value ?? "");

  const activeValue = controlledValue ?? internalValue;

  const handleSelect = useCallback(
    (val: string) => {
      if (controlledValue === undefined) {
        setInternalValue(val);
      }
      onChange?.(val);
    },
    [controlledValue, onChange],
  );

  return (
    <View role="tablist" className={twMerge(containerStyles[variant], className)}>
      {items.map((item) => {
        const isActive = activeValue === item.value;
        return (
          <Pressable
            key={item.value}
            role="tab"
            aria-selected={isActive}
            onPress={() => handleSelect(item.value)}
            className={twMerge(
              tabBaseStyles[variant],
              isActive && tabActiveStyles[variant],
              itemClassName,
            )}
          >
            {typeof item.label === "string" ? (
              <Text
                className={twMerge(
                  "font-medium",
                  variant === "segmented" ? "text-[12px]" : "text-[13px]",
                  isActive ? "text-fg-primary" : "text-fg-tertiary",
                )}
              >
                {item.label}
              </Text>
            ) : (
              item.label
            )}
            {variant === "underline" && isActive && (
              <View className="absolute left-0 right-0 h-px bg-fg-primary" style={{ bottom: -1 }} />
            )}
          </Pressable>
        );
      })}
    </View>
  );
}
