import { useId } from "react";
import { Text, View } from "react-native";
import { twMerge, useApp } from "@left-curve/foundation";
import { Decimal, formatDisplayNumber } from "@left-curve/dango/utils";

import type { FormatNumberOptions } from "@left-curve/dango/utils";

export type FormattedNumberProps = {
  readonly value: string | number;
  readonly formatOptions?: Partial<FormatNumberOptions>;
  readonly className?: string;
  /** Show +/- sign prefix. */
  readonly sign?: boolean;
  /** Color green/red based on sign (uses text-up / text-down). */
  readonly colorSign?: boolean;
};

export function FormattedNumber({
  value,
  formatOptions,
  className,
  sign = false,
  colorSign = false,
}: FormattedNumberProps) {
  const id = useId();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const parts = formatDisplayNumber(value, { ...formatNumberOptions, ...formatOptions });

  const isPositive = Decimal(value).gte(0);
  const signPrefix = sign ? (isPositive ? "+" : "") : "";

  const colorClass = colorSign ? (isPositive ? "text-up" : "text-down") : undefined;

  // Extract text color classes from className so they can be applied directly to
  // Text elements (React Native Web Text does not inherit color from parent Views).
  const textColorFromParent =
    className
      ?.split(/\s+/)
      .filter(
        (cls) =>
          cls.startsWith("text-fg-") ||
          cls.startsWith("text-up") ||
          cls.startsWith("text-down") ||
          cls.startsWith("text-accent") ||
          cls.startsWith("text-white") ||
          cls.startsWith("text-btn-"),
      )
      .join(" ") || undefined;

  const resolvedTextColor = colorClass ?? textColorFromParent;

  return (
    <View className={twMerge("flex flex-row items-baseline", className)}>
      {signPrefix ? (
        <Text className={twMerge("font-mono tabular-nums", resolvedTextColor)}>{signPrefix}</Text>
      ) : null}
      {parts.map((part, index) => {
        if (part.type === "subscript") {
          return (
            <Text
              key={`${id}-${
                // biome-ignore lint/suspicious/noArrayIndexKey: parts are positional
                index
              }`}
              className={twMerge("font-mono tabular-nums text-[0.65em]", resolvedTextColor)}
              style={{ lineHeight: 14 }}
            >
              {part.value}
            </Text>
          );
        }

        return (
          <Text
            key={`${id}-${
              // biome-ignore lint/suspicious/noArrayIndexKey: parts are positional
              index
            }`}
            className={twMerge("font-mono tabular-nums", resolvedTextColor)}
          >
            {part.value}
          </Text>
        );
      })}
    </View>
  );
}
