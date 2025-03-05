import * as React from "react";
import { type VariantProps, tv } from "tailwind-variants";
import { twMerge } from "../../utils";

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
  errorMessage?: string;
  hintMessage?: string;
  classNames?: {
    base?: string;
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
      isDisabled,
      fullWidth,
      startText,
      hintMessage,
      errorMessage,
      label,
      name,
      placeholder,
      ...props
    },
    ref,
  ) => {
    const isInvalid = errorMessage ? true : invalid;
    const { base, input, inputWrapper } = inputVariants({
      fullWidth,
      isDisabled,
      isInvalid,
    });
    return (
      <div className={base({ className: classNames?.base })}>
        {label ? (
          <label className="exposure-sm-italic text-gray-700" htmlFor={name}>
            {label}
          </label>
        ) : null}

        <div className={inputWrapper({ className: classNames?.inputWrapper })}>
          {startContent ? startContent : null}
          {!props.value && placeholder ? (
            <div className="w-full absolute z-0 text-gray-500">{placeholder}</div>
          ) : null}
          <input
            type={type}
            disabled={isDisabled}
            className={input({ startText, className: classNames?.input })}
            ref={ref}
            name={name}
            {...props}
          />
          {endContent ? endContent : null}
        </div>

        <div
          className={twMerge("hidden text-left", {
            block: errorMessage,
          })}
        >
          <span className="diatype-sm-regular text-error-500">{errorMessage}</span>
        </div>

        <div
          className={twMerge("hidden", {
            block: !bottomComponent && hintMessage,
          })}
        >
          <span className="diatype-sm-regular text-gray-500">{hintMessage}</span>
        </div>

        {bottomComponent ? (
          <div className="text-gray-500 diatype-sm-regular">{bottomComponent}</div>
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
        "relative w-full inline-flex tap-highlight-transparent flex-row items-center shadow-input-shadow gap-3 z-10",
        "bg-rice-25 hover:bg-rice-50 border border-transparent active:border-rice-200",
        "px-4 py-[13px] rounded-lg h-[46px]",
      ],
      input: [
        "flex-1 diatype-m-regular bg-transparent !outline-none placeholder:text-gray-400 text-gray-700 leading-none",
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
            "pointer-events-none bg-gray-50 placeholder:text-gray-300 text-gray-300 active:border-transparent",
          label: "pointer-events-none",
        },
      },
      isInvalid: {
        true: {
          inputWrapper: "border-error-500",
          input: "text-gray-700",
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
