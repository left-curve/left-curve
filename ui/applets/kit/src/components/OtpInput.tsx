/** biome-ignore-all lint/suspicious/noArrayIndexKey: <explanation> */
import { twMerge } from "@left-curve/foundation";
import React from "react";

type OtpInputProps = {
  length?: number;
  value?: string;
  onChange?: (code: string) => void;
  isInvalid?: boolean;
  autoFocus?: boolean;
  disabled?: boolean;
};

export function OtpInput({
  length = 4,
  value,
  onChange,
  isInvalid = false,
  autoFocus = true,
  disabled = false,
}: OtpInputProps) {
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
              aria-invalid={isInvalid || undefined}
              className={twMerge(
                "peer w-14 h-14 text-center rounded-xl diatype-lg-bold outline-none border-2 z-10 relative bg-surface-secondary-rice",
                isInvalid
                  ? "border-status-fail !text-status-fail"
                  : "border-surface-quaternary-rice",
              )}
            />
            <span
              className={twMerge(
                "transition-all scale-0 peer-focus:opacity-100 peer-focus:scale-100 w-[62px] h-[62px] top-[-3px] left-[-3px] absolute border-2 rounded-2xl",
                isInvalid ? "border-status-fail" : "border-surface-tertiary-rice",
              )}
            />
          </div>
        );
      })}
    </div>
  );
}
