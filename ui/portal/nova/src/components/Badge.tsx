import { type ReactNode, Children, isValidElement } from "react";
import { View, Text, type ViewProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

type BadgeVariant = "default" | "up" | "down" | "warn" | "accent" | "outline";

export type BadgeProps = ViewProps & {
  variant?: BadgeVariant;
  children: ReactNode;
};

const containerStyles: Record<BadgeVariant, string> = {
  default: "bg-bg-tint border-border-subtle",
  up: "bg-up-bg border-transparent",
  down: "bg-down-bg border-transparent",
  warn: "border-transparent",
  accent: "bg-accent-bg border-transparent",
  outline: "bg-transparent border-border-strong",
};

const textStyles: Record<BadgeVariant, string> = {
  default: "text-fg-secondary",
  up: "text-up",
  down: "text-down",
  warn: "text-warn",
  accent: "text-accent",
  outline: "text-fg-secondary",
};

const WARN_BG = "color-mix(in oklch, var(--color-warn) 18%, transparent)";

function wrapChildren(children: ReactNode, variant: BadgeVariant): ReactNode {
  const textClass = twMerge("text-[11px] font-medium tracking-tight", textStyles[variant]);

  return Children.map(children, (child) => {
    if (typeof child === "string" || typeof child === "number") {
      return <Text className={textClass}>{child}</Text>;
    }
    if (isValidElement(child) && child.type === Text) {
      const existing = (child.props as { className?: string }).className;
      return (
        <Text {...(child.props as object)} className={twMerge(textClass, existing)}>
          {(child.props as { children?: ReactNode }).children}
        </Text>
      );
    }
    return child;
  });
}

export function Badge({ variant = "default", className, style, children, ...props }: BadgeProps) {
  const warnStyle = variant === "warn" ? { backgroundColor: WARN_BG } : undefined;

  return (
    <View
      className={twMerge(
        "inline-flex items-center gap-1",
        "h-5 px-[7px]",
        "rounded-full border",
        "tabular-nums",
        containerStyles[variant],
        className,
      )}
      style={warnStyle ? [warnStyle, style] : style}
      {...props}
    >
      {wrapChildren(children, variant)}
    </View>
  );
}
