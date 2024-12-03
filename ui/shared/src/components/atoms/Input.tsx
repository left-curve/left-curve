import * as React from "react";
import { type VariantProps, tv } from "tailwind-variants";
import { twMerge } from "../../utils";

export interface InputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "size" | "color" | "className">,
    VariantProps<typeof inputVariants> {
  startContent?: React.ReactNode;
  endContent?: React.ReactNode;
  bottomComponent?: React.ReactNode;
  errorMessage?: string;
  validMessage?: string;
  classNames?: {
    base?: string;
    inputWrapper?: string;
    input?: string;
    description?: string;
  };
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    {
      classNames,
      startContent,
      endContent,
      bottomComponent,
      type,
      size,
      color,
      isInvalid: invalid,
      isDisabled,
      fullWidth,
      startText,
      validMessage,
      errorMessage,
      ...props
    },
    ref,
  ) => {
    const isInvalid = errorMessage ? true : invalid;
    const isValid = !isInvalid && (!!validMessage || !!props.value);
    const { base, input, inputWrapper } = inputVariants({
      color,
      size,
      fullWidth,
      isDisabled,
      isInvalid,
      isValid,
    });
    return (
      <div className={base({ className: classNames?.base })}>
        <div className={inputWrapper({ className: classNames?.inputWrapper })}>
          {startContent ? startContent : null}
          <input
            type={type}
            disabled={isDisabled}
            className={input({ startText, className: classNames?.input })}
            ref={ref}
            {...props}
          />
          {endContent ? endContent : null}
        </div>

        <div
          className={twMerge("h-0 translate-y-[-2rem] transition-all duration-500", {
            "px-6 h-6 translate-y-0": !bottomComponent && errorMessage,
          })}
        >
          <span className="text-typography-pink-400 typography-caption-m">{errorMessage}</span>
        </div>

        <div
          className={twMerge("h-0 translate-y-[-2rem] transition-all duration-500", {
            "px-6 h-6 translate-y-0": !bottomComponent && validMessage,
          })}
        >
          <span className="text-typography-green-400 typography-caption-m">{validMessage}</span>
        </div>

        {bottomComponent ? bottomComponent : null}
      </div>
    );
  },
);

Input.displayName = "Input";

export { Input };

const inputVariants = tv(
  {
    slots: {
      base: "group flex flex-col data-[hidden=true]:hidden gap-1",
      inputWrapper:
        "relative w-full inline-flex tap-highlight-transparent flex-row items-center shadow-sm px-6 py-3 gap-3 z-10",
      input: [
        "flex-1 font-normal bg-transparent !outline-none placeholder:text-foreground-500 focus:outline-none min-w-0",
        "data-[has-start-content=true]:ps-1.5",
        "data-[has-end-content=true]:pe-1.5",
        "file:cursor-pointer file:bg-transparent file:border-0",
        "autofill:bg-transparent bg-clip-text",
      ],
    },
    variants: {
      color: {
        default: {},
        purple: {},
      },
      size: {
        sm: {},
        md: {
          inputWrapper: "min-h-12 rounded-xl",
          input: "text-base",
        },
        lg: {
          inputWrapper: "min-h-14 rounded-2xl",
          input: "text-base",
        },
      },
      isDisabled: {
        true: {
          base: "opacity-disabled pointer-events-none",
          inputWrapper: "pointer-events-none",
          label: "pointer-events-none",
        },
      },
      isValid: {
        true: {
          inputWrapper: "border-2 border-borders-rose-600",
        },
      },
      isInvalid: {
        true: {
          inputWrapper: "border-2 border-borders-pink-300",
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
      color: "default",
      size: "md",
      fullWidth: true,
      isDisabled: false,
      startText: "left",
    },
    compoundVariants: [
      {
        size: "md",
        color: "default",
        class: {
          inputWrapper:
            "bg-surface-rose-300 text-typography-rose-600 group-hover:bg-surface-rose-400",
          input: "placeholder:text-typography-rose-600 focus:text-typography-black-100",
        },
      },
      {
        size: "lg",
        color: "default",
        class: {
          inputWrapper:
            "bg-surface-rose-300 text-typography-rose-600 group-hover:bg-surface-rose-400",
          input: "placeholder:text-typography-rose-600 focus:text-typography-black-100",
        },
      },
      {
        size: "md",
        color: "purple",
        class: {
          inputWrapper:
            "bg-surface-purple-100 text-typography-black-300 group-hover:bg-surface-purple-200 border border-purple-600/40",
          input: "placeholder:text-typography-black-100/40 focus:text-typography-black-100",
        },
      },
      {
        size: "lg",
        color: "purple",
        class: {
          inputWrapper:
            "bg-surface-purple-100 text-typography-black-300 group-hover:bg-surface-purple-200 border border-purple-600/40",
          input: "placeholder:text-typography-black-100/40 focus:text-typography-black-100",
        },
      },
    ],
  },
  {
    twMerge: true,
  },
);
