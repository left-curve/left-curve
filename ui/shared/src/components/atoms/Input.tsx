import * as React from "react";
import { type VariantProps, tv } from "tailwind-variants";

export interface InputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "size" | "color" | "className">,
    VariantProps<typeof inputVariants> {
  startContent?: React.ReactNode;
  endContent?: React.ReactNode;
  bottomComponent?: React.ReactNode;
  error?: string;
  description?: string;
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
      isDisabled,
      fullWidth,
      startText,
      error,
      description: descriptionText,
      ...props
    },
    ref,
  ) => {
    const { base, input, inputWrapper, description } = inputVariants({
      color,
      size,
      fullWidth,
      isDisabled,
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
        {!bottomComponent || error ? (
          <div className={description({ className: classNames?.description })}>
            {error ? <span className="text-typography-pink-300">{error}</span> : null}
            {descriptionText && !error ? <span>{descriptionText}</span> : null}
          </div>
        ) : null}
        {bottomComponent && !error ? bottomComponent : null}
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
        "relative w-full inline-flex tap-highlight-transparent flex-row items-center shadow-sm px-6 py-3 gap-3",
      input: [
        "flex-1 font-normal bg-transparent !outline-none placeholder:text-foreground-500 focus:outline-none min-w-0",
        "data-[has-start-content=true]:ps-1.5",
        "data-[has-end-content=true]:pe-1.5",
        "file:cursor-pointer file:bg-transparent file:border-0",
        "autofill:bg-transparent bg-clip-text",
      ],
      description: "h-5 px-6 text-[12px] font-semibold",
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
          description: "text-typography-600",
        },
      },
      {
        size: "lg",
        color: "default",
        class: {
          inputWrapper:
            "bg-surface-rose-300 text-typography-rose-600 group-hover:bg-surface-rose-400",
          input: "placeholder:text-typography-rose-600 focus:text-typography-black-100",
          description: "text-typography-600",
        },
      },
      {
        size: "md",
        color: "purple",
        class: {
          inputWrapper:
            "bg-surface-purple-100 text-typography-black-300 group-hover:bg-surface-purple-200 border border-purple-600/40",
          input: "placeholder:text-typography-black-100/40 focus:text-typography-black-100",
          description: "text-typography-black-300",
        },
      },
      {
        size: "lg",
        color: "purple",
        class: {
          inputWrapper:
            "bg-surface-purple-100 text-typography-black-300 group-hover:bg-surface-purple-200 border border-purple-600/40",
          input: "placeholder:text-typography-black-100/40 focus:text-typography-black-100",
          description: "text-typography-black-300",
        },
      },
    ],
  },
  {
    twMerge: true,
  },
);
