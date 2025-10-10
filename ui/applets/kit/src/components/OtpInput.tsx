/** biome-ignore-all lint/suspicious/noArrayIndexKey: array index can be safe use*/
import { twMerge } from "@left-curve/foundation";
import React from "react";

interface OtpInputProps
  extends Omit<
    React.InputHTMLAttributes<HTMLInputElement>,
    | "value"
    | "defaultValue"
    | "onChange"
    | "maxLength"
    | "type"
    | "size"
    | "inputMode"
    | "autoComplete"
  > {
  length?: number;
  value?: string;
  onChange?: (code: string) => void;
  autoFocus?: boolean;
  disabled?: boolean;
  errorMessage?: string;
}

export const OtpInput = React.forwardRef<HTMLDivElement, OtpInputProps>(
  (
    {
      length = 4,
      value,
      onChange,
      autoFocus = true,
      disabled = false,
      className,
      errorMessage,
      ...inputAttrs
    },
    _,
  ) => {
    const regexValidation = /\D/g;
    const [inner, setInner] = React.useState<string>("".padEnd(length, " "));
    const code = (value ?? inner).padEnd(length, " ").slice(0, length);

    const refs = React.useRef<HTMLInputElement[]>(
      Array.from({ length }, () => document.createElement("input")),
    );

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
      refs.current[i]?.select();
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
      else refs.current[length - 1]?.blur();
    };

    const handleKeyDown = (i: number, e: React.KeyboardEvent<HTMLInputElement>) => {
      const key = e.key;
      if (key === "Backspace") {
        e.preventDefault();
        let next = code;
        if (code[i].trim() !== "") {
          next = setCharAt(code, i, " ");
          update(next);
          focusIndex(i);
        } else {
          const prev = i - 1;
          if (prev >= 0) {
            next = setCharAt(code, prev, " ");
            update(next);
            focusIndex(prev);
          }
        }
        return;
      }
      if (key === "ArrowLeft") {
        e.preventDefault();
        focusIndex(i - 1);
      }
      if (key === "ArrowRight") {
        e.preventDefault();
        focusIndex(i + 1);
      }
      if (key === "Home") {
        e.preventDefault();
        focusIndex(0);
      }
      if (key === "End") {
        e.preventDefault();
        focusIndex(length - 1);
      }
    };

    const handlePaste = (i: number, e: React.ClipboardEvent<HTMLInputElement>) => {
      e.preventDefault();
      const pasted = e.clipboardData.getData("text").replace(regexValidation, "");
      if (pasted) handleChange(i, pasted);
    };

    React.useEffect(() => {
      if (autoFocus) focusIndex(0);
    }, []);

    return (
      <div className="flex flex-col gap-1 items-center">
        <div className="inline-flex gap-3">
          {Array.from({ length }).map((_, i) => {
            const hasValue = code[i]?.trim() !== "";
            return (
              <div className="relative" key={`input-otp-${i}`}>
                <input
                  ref={(el) => {
                    if (el) refs.current[i] = el;
                  }}
                  value={hasValue ? code[i] : ""}
                  onChange={(e) => handleChange(i, e.target.value)}
                  onKeyDown={(e) => handleKeyDown(i, e)}
                  onPaste={(e) => handlePaste(i, e)}
                  inputMode="numeric"
                  autoComplete="one-time-code"
                  maxLength={1}
                  disabled={disabled}
                  aria-label={`Digit ${i + 1} of the verification code`}
                  className={twMerge(
                    "peer w-12 h-12 text-center rounded-sm h2-medium outline-none border-2 z-10 relative bg-surface-secondary-rice focus:border-primitives-blue-light-500 shadow-account-card",
                    errorMessage ? "border-status-fail" : "border-transparent",
                    hasValue ? "bg-surface-tertiary-rice" : "bg-surface-secondary-rice",
                    className,
                  )}
                  {...inputAttrs}
                />
              </div>
            );
          })}
        </div>
        <div
          className={twMerge("hidden", {
            block: errorMessage,
          })}
        >
          <span className="diatype-sm-regular text-status-fail">{errorMessage}</span>
        </div>
      </div>
    );
  },
);
OtpInput.displayName = "OtpInput";
