import { useState, useCallback } from "react";
import { Pressable, View, type PressableProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

export type ToggleProps = Omit<PressableProps, "onPress"> & {
  checked?: boolean;
  defaultChecked?: boolean;
  onChange?: (checked: boolean) => void;
};

export function Toggle({
  checked: controlledChecked,
  defaultChecked = false,
  onChange,
  className,
  disabled,
  ...props
}: ToggleProps) {
  const [internalChecked, setInternalChecked] = useState(defaultChecked);
  const isChecked = controlledChecked ?? internalChecked;

  const handleToggle = useCallback(() => {
    const next = !isChecked;
    if (controlledChecked === undefined) {
      setInternalChecked(next);
    }
    onChange?.(next);
  }, [isChecked, controlledChecked, onChange]);

  return (
    <Pressable
      role="switch"
      aria-checked={isChecked}
      onPress={handleToggle}
      disabled={disabled}
      className={twMerge(
        "relative shrink-0 overflow-hidden",
        "w-8 h-[18px]",
        "rounded-full border",
        "cursor-pointer select-none appearance-none",
        "transition-[background,border-color] duration-200 ease-[var(--ease)]",
        "disabled:opacity-40 disabled:cursor-not-allowed",
        isChecked ? "bg-btn-primary-bg border-btn-primary-bg" : "bg-bg-tint border-border-default",
        className,
      )}
      {...props}
    >
      <View
        className={twMerge(
          "absolute top-[1px] left-[1px]",
          "w-3.5 h-3.5 rounded-full",
          "shadow-sm",
          "transition-transform duration-200 ease-[var(--ease)]",
          isChecked ? "translate-x-3.5 bg-btn-primary-fg" : "translate-x-0 bg-bg-elev",
        )}
      />
    </Pressable>
  );
}
