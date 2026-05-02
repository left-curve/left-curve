import { type ReactNode } from "react";
import { Pressable, Text, type PressableProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

type ButtonVariant = "primary" | "secondary" | "ghost" | "up" | "down";
type ButtonSize = "sm" | "default" | "lg";

export type ButtonProps = PressableProps & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  iconOnly?: boolean;
  children: ReactNode;
};

const variantStyles: Record<ButtonVariant, string> = {
  primary: "bg-btn-primary-bg hover:bg-btn-primary-hover",
  secondary: "bg-bg-surface border-border-default hover:bg-bg-tint hover:border-border-strong",
  ghost: "bg-transparent hover:bg-bg-tint",
  up: "bg-up hover:brightness-105",
  down: "bg-down hover:brightness-105",
};

const variantTextStyles: Record<ButtonVariant, string> = {
  primary: "text-btn-primary-fg",
  secondary: "text-fg-primary",
  ghost: "text-fg-secondary",
  up: "text-white",
  down: "text-white",
};

const sizeStyles: Record<ButtonSize, string> = {
  sm: "h-7 px-2.5 text-[12px]",
  default: "h-8 px-3.5 text-[13px]",
  lg: "h-11 px-[18px] text-[14px]",
};

export function Button({
  variant = "primary",
  size = "default",
  iconOnly = false,
  className,
  children,
  ...props
}: ButtonProps) {
  const textColor = variantTextStyles[variant];

  return (
    <Pressable
      role="button"
      className={twMerge(
        "inline-flex items-center justify-center gap-1.5",
        "border border-transparent",
        "rounded-btn",
        "font-medium tracking-flat",
        "whitespace-nowrap select-none",
        "transition-[background,border-color,color,transform] duration-150 ease-[var(--ease)]",
        "active:translate-y-[0.5px]",
        "disabled:opacity-50 disabled:cursor-not-allowed disabled:pointer-events-none",
        variantStyles[variant],
        sizeStyles[size],
        iconOnly && "w-[var(--btn-h,32px)] px-0",
        className,
      )}
      {...props}
    >
      {typeof children === "string" ? <Text className={textColor}>{children}</Text> : children}
    </Pressable>
  );
}
