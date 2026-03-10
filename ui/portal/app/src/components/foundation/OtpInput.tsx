/** biome-ignore-all lint/suspicious/noArrayIndexKey: <explanation> */
import type React from "react";
import { forwardRef, useEffect, useMemo, useRef, useState } from "react";
import { TextInput, View } from "react-native";
import { twMerge } from "@left-curve/foundation";

import { GlobalText } from "./GlobalText";
import { useTheme } from "~/hooks/useTheme";

type OtpInputProps = Omit<
  React.ComponentPropsWithoutRef<typeof TextInput>,
  | "value"
  | "defaultValue"
  | "onChangeText"
  | "maxLength"
  | "inputMode"
  | "autoComplete"
  | "secureTextEntry"
> & {
  length?: number;
  value?: string;
  onChange?: (code: string) => void;
  autoFocus?: boolean;
  disabled?: boolean;
  errorMessage?: string;
  containerClassName?: string;
  inputClassName?: string;
};

export const OtpInput = forwardRef<View, OtpInputProps>(
  (
    {
      length = 4,
      value,
      onChange,
      autoFocus = true,
      disabled = false,
      className,
      containerClassName,
      inputClassName,
      errorMessage,
      ...inputAttrs
    },
    ref,
  ) => {
    const { theme } = useTheme();
    const regexValidation = useMemo(() => /\D/g, []);
    const [inner, setInner] = useState<string>("".padEnd(length, " "));
    const code = (value ?? inner).padEnd(length, " ").slice(0, length);

    const refs = useRef<Array<TextInput | null>>(Array.from({ length }, () => null));

    useEffect(() => {
      setInner("".padEnd(length, " "));
    }, [length]);

    const setCharAt = (str: string, idx: number, ch: string) =>
      str.substring(0, idx) + ch + str.substring(idx + 1);

    const update = (next: string) => {
      onChange?.(next.replaceAll(" ", ""));
      setInner(next);
    };

    const focusIndex = (i: number) => {
      if (i < 0) i = 0;
      if (i > length - 1) i = length - 1;
      refs.current[i]?.focus();
    };

    const handleChange = (i: number, raw: string) => {
      const cleaned = raw.replace(regexValidation, "");
      if (!cleaned) return;

      let next = code;
      let cursor = i;
      for (const ch of cleaned) {
        if (cursor >= length) break;
        next = setCharAt(next, cursor, ch);
        cursor++;
      }
      update(next);
      if (cursor <= length - 1) focusIndex(cursor);
    };

    const handleBackspace = (i: number) => {
      let next = code;
      if (code[i].trim() !== "") {
        next = setCharAt(code, i, " ");
        update(next);
        focusIndex(i);
        return;
      }
      const prev = i - 1;
      if (prev >= 0) {
        next = setCharAt(code, prev, " ");
        update(next);
        focusIndex(prev);
      }
    };

    useEffect(() => {
      if (autoFocus) focusIndex(0);
    }, [autoFocus]);

    return (
      <View ref={ref} className={twMerge("flex flex-col gap-2 items-center", containerClassName)}>
        <View className={twMerge("flex flex-row gap-3", className)}>
          {Array.from({ length }).map((_, i) => {
            const hasValue = code[i]?.trim() !== "";
            return (
              <View className="relative" key={`otp-${i}`}>
                <TextInput
                  ref={(el) => {
                    refs.current[i] = el;
                  }}
                  value={hasValue ? code[i] : ""}
                  onChangeText={(text) => handleChange(i, text)}
                  onKeyPress={(e) => {
                    if (e.nativeEvent.key === "Backspace") {
                      e.preventDefault?.();
                      handleBackspace(i);
                    }
                  }}
                  keyboardType="number-pad"
                  textContentType="oneTimeCode"
                  maxLength={1}
                  editable={!disabled}
                  placeholder=""
                  selectTextOnFocus
                  selectionColor={theme === "dark" ? "#6A5D42" : "#EFDAA4"}
                  className={twMerge(
                    "w-12 h-12 text-center rounded-sm h2-medium border-2 bg-surface-secondary-rice text-ink-secondary-700",
                    "focus:border-primitives-blue-light-500",
                    errorMessage ? "border-status-fail" : "border-transparent",
                    hasValue ? "bg-surface-tertiary-rice" : "bg-surface-secondary-rice",
                    inputClassName,
                  )}
                  {...inputAttrs}
                />
              </View>
            );
          })}
        </View>
        {errorMessage ? (
          <GlobalText className="diatype-sm-regular text-status-fail">{errorMessage}</GlobalText>
        ) : null}
      </View>
    );
  },
);

OtpInput.displayName = "OtpInput";
