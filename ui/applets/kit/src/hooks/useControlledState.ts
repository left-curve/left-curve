import { useState } from "react";

export function useControlledState<T>(
  value?: T,
  onChange?: (value: T) => void,
  defaultValue?: T | (() => T),
): [T, (newValue: T) => void] {
  const resolvedDefaultValue =
    typeof defaultValue === "function" ? (defaultValue as () => T)() : defaultValue;

  const [internalValue, setInternalValue] = useState<T>(
    (value ?? resolvedDefaultValue) || ({} as T),
  );

  const isControlled = value !== undefined;

  const handleChange = (newValue: T) => {
    if (!isControlled) {
      setInternalValue(newValue);
    }

    onChange?.(newValue);
  };

  return [isControlled ? value! : internalValue, handleChange];
}
