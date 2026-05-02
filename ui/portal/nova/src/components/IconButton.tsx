import { type ReactNode } from "react";
import { Pressable, Text, type PressableProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

type IconButtonSize = "sm" | "default" | "lg";
type IconButtonShape = "circle" | "square";

export type IconButtonProps = PressableProps & {
  size?: IconButtonSize;
  shape?: IconButtonShape;
  children: ReactNode;
};

const sizeStyles: Record<IconButtonSize, string> = {
  sm: "w-7 h-7",
  default: "w-8 h-8",
  lg: "w-11 h-11",
};

export function IconButton({
  size = "default",
  shape = "square",
  className,
  children,
  ...props
}: IconButtonProps) {
  return (
    <Pressable
      role="button"
      className={twMerge(
        "inline-flex items-center justify-center",
        "bg-transparent text-fg-secondary",
        "border border-transparent",
        "hover:bg-bg-tint hover:text-fg-primary",
        "transition-[background,color] duration-150 ease-[var(--ease)]",
        "active:translate-y-[0.5px]",
        "disabled:opacity-50 disabled:cursor-not-allowed disabled:pointer-events-none",
        sizeStyles[size],
        shape === "circle" ? "rounded-full" : "rounded-btn",
        className,
      )}
      {...props}
    >
      {typeof children === "string" ? <Text>{children}</Text> : children}
    </Pressable>
  );
}
