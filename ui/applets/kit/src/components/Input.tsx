import * as React from "react";
import { type VariantProps, tv } from "tailwind-variants";
import { twMerge } from "#utils/twMerge.js";
import { Skeleton } from "./Skeleton";

export interface InputProps
  extends Omit<
      React.InputHTMLAttributes<HTMLInputElement>,
      "placeholder" | "size" | "color" | "className"
    >,
    VariantProps<typeof inputVariants> {
  label?: string;
  startContent?: React.ReactNode;
  endContent?: React.ReactNode;
  bottomComponent?: React.ReactNode;
  insideBottomComponent?: React.ReactNode;
  errorMessage?: string;
  hideErrorMessage?: boolean;
  hintMessage?: string;
  isLoading?: boolean;
  classNames?: {
    base?: string;
    inputParent?: string;
    inputWrapper?: string;
    input?: string;
    description?: string;
  };
  placeholder?: React.ReactNode;
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    {
      classNames,
      startContent,
      endContent,
      bottomComponent,
      type,
      isInvalid: invalid,
      isLoading,
      isDisabled,
      fullWidth,
      startText,
      hintMessage,
      errorMessage,
      insideBottomComponent,
      label,
      name,
      placeholder,
      hideErrorMessage,
      onFocus,
      onBlur,
      ...props
    },
    ref,
  ) => {
    const isInvalid = errorMessage ? true : invalid;
    const [isFocus, setIsFocus] = React.useState(false);
    const { base, input, inputWrapper, inputParent } = inputVariants({
      fullWidth,
      isDisabled,
      isInvalid,
    });

    return (
      <div className={base({ className: classNames?.base })}>
        {label ? (
          <label className="exposure-sm-italic text-secondary-700" htmlFor={name}>
            {label}
          </label>
        ) : null}

        <div
          className={twMerge(inputWrapper({ className: classNames?.inputWrapper }), {
            group: isFocus,
          })}
          data-focus={isFocus}
        >
          <div className={inputParent({ className: classNames?.inputParent })}>
            {startContent ? startContent : null}
            <div className="relative flex-1 flex items-center">
              {!props.value && !isLoading && placeholder ? (
                <div
                  className={twMerge("w-full absolute z-0 text-tertiary-500 text-left ", {
                    "text-right": startText === "right",
                  })}
                >
                  {placeholder}
                </div>
              ) : null}
              {isLoading ? (
                <Skeleton className="h-8 w-full" />
              ) : (
                <input
                  type={type}
                  onFocus={(e) => {
                    setIsFocus(true);
                    onFocus?.(e);
                  }}
                  onBlur={(e) => {
                    setIsFocus(false);
                    onBlur?.(e);
                  }}
                  disabled={isDisabled}
                  className={input({ startText, className: classNames?.input })}
                  ref={ref}
                  name={name}
                  size={1}
                  {...props}
                />
              )}
            </div>
            {endContent ? endContent : null}
          </div>
          {insideBottomComponent ? insideBottomComponent : null}
        </div>

        <div
          className={twMerge("hidden text-left", {
            block: errorMessage && !hideErrorMessage,
          })}
        >
          <span className="diatype-sm-regular text-error-500">{errorMessage}</span>
        </div>

        <div
          className={twMerge("hidden", {
            block: !bottomComponent && hintMessage,
          })}
        >
          <span className="diatype-sm-regular text-tertiary-500">{hintMessage}</span>
        </div>

        {bottomComponent ? (
          <div className="text-tertiary-500 diatype-sm-regular">{bottomComponent}</div>
        ) : null}
      </div>
    );
  },
);

Input.displayName = "Input";

export { Input };

const inputVariants = tv(
  {
    slots: {
      base: " flex flex-col data-[hidden=true]:hidden gap-1 relative",
      inputWrapper: [
        "relative w-full inline-flex tap-highlight-transparent flex-row items-center shadow-account-card  gap-2 z-10",
        "bg-surface-secondary-rice hover:bg-surface-tertiary-rice border border-transparent active:border-surface-quaternary-rice",
        "px-4 py-[13px] rounded-lg h-[46px]",
      ],
      inputParent: "w-full inline-flex relative items-center gap-2",
      input: [
        "flex-1 diatype-m-regular bg-transparent !outline-none placeholder:text-tertiary-500 text-secondary-700 leading-none relative z-10",
        "data-[has-start-content=true]:ps-1.5",
        "data-[has-end-content=true]:pe-1.5",
        "file:cursor-pointer file:bg-transparent file:border-0",
        "autofill:bg-transparent bg-clip-text z-10",
      ],
    },
    variants: {
      isDisabled: {
        true: {
          base: "opacity-disabled pointer-events-none",
          inputWrapper:
            "pointer-events-none bg-surface-disabled-gray placeholder:text-foreground-disabled-gray text-foreground-disabled-gray active:border-transparent",
          label: "pointer-events-none",
        },
      },
      isInvalid: {
        true: {
          inputWrapper: "border-error-500",
          input: "text-secondary-700",
        },
      },
      startText: {
        left: {},
        right: {
          input: "text-end",
        },
      },
      fullWidth: {
        true: {
          base: "w-full",
        },
      },
    },
    defaultVariants: {
      fullWidth: true,
      isDisabled: false,
      startText: "left",
    },
  },
  {
    twMerge: true,
  },
);
