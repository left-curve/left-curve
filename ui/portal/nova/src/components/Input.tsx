import { type ReactNode } from "react";
import { View, Text, TextInput, type TextInputProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

export type InputProps = TextInputProps & {
  label?: string;
  error?: string;
  prefix?: ReactNode;
  suffix?: ReactNode;
  wrapperClassName?: string;
  disabled?: boolean;
};

export function Input({
  label,
  error,
  prefix,
  suffix,
  className,
  wrapperClassName,
  disabled,
  ...props
}: InputProps) {
  return (
    <View className={twMerge("flex flex-col gap-1", wrapperClassName)}>
      {label && (
        <Text className="text-[11px] text-fg-tertiary tracking-wide uppercase font-medium">
          {label}
        </Text>
      )}
      <View
        className={twMerge(
          "flex flex-row items-center",
          "h-[var(--field-h,36px)] px-3",
          "bg-bg-surface",
          "border rounded-field",
          "text-[13px] text-fg-primary",
          "transition-[border-color,background] duration-150 ease-[var(--ease)]",
          error
            ? "border-down"
            : "border-border-default hover:border-border-strong focus-within:border-fg-primary focus-within:bg-bg-elev",
          disabled && "opacity-55 bg-bg-sunk pointer-events-none",
        )}
      >
        {prefix && <View className="text-fg-tertiary text-[12px] shrink-0 mr-2">{prefix}</View>}
        <TextInput
          className={twMerge(
            "flex-1 min-w-0 h-full",
            "bg-transparent border-0 outline-none",
            "text-fg-primary tabular-nums",
            "placeholder:text-fg-quaternary",
            className,
          )}
          editable={!disabled}
          {...props}
        />
        {suffix && <View className="text-fg-tertiary text-[12px] shrink-0 ml-2">{suffix}</View>}
      </View>
      {error && <Text className="text-[11px] text-down">{error}</Text>}
    </View>
  );
}
