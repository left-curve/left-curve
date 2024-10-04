import * as React from "react";
import { type VariantProps, tv } from "tailwind-variants";

export interface InputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "size" | "color">,
    VariantProps<typeof inputVariants> {
  startContent?: React.ReactNode;
  endContent?: React.ReactNode;
  bottomComponent?: React.ReactNode;
  error?: string;
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    {
      className,
      startContent,
      endContent,
      bottomComponent,
      type,
      size,
      color,
      disabled,
      fullWidth,
      startText,
      error,
      ...props
    },
    ref,
  ) => {
    const { base, input, inputWrapper, description } = inputVariants({
      className,
      color,
      size,
      fullWidth,
      isDisabled: disabled,
    });
    return (
      <div className={base()}>
        <div className={inputWrapper()}>
          {startContent ? startContent : null}
          <input type={type} className={input({ startText })} ref={ref} {...props} />
          {endContent ? endContent : null}
        </div>
        {error ? <span className="text-danger-500">{error}</span> : null}
        {bottomComponent ? <span className={description()}>{bottomComponent}</span> : null}
      </div>
    );
  },
);

Input.displayName = "Input";

export { Input };

const inputVariants = tv(
  {
    slots: {
      base: "group flex flex-col data-[hidden=true]:hidden",
      inputWrapper:
        "relative w-full inline-flex tap-highlight-transparent flex-row items-center shadow-sm px-6 py-3 gap-3",
      input: [
        "flex-1 font-normal bg-transparent !outline-none placeholder:text-foreground-500 focus-visible:outline-none",
        "data-[has-start-content=true]:ps-1.5",
        "data-[has-end-content=true]:pe-1.5",
        "file:cursor-pointer file:bg-transparent file:border-0",
        "autofill:bg-transparent bg-clip-text",
      ],
      description: "text-sm",
    },
    variants: {
      color: {
        default: {},
      },
      size: {
        sm: {},
        md: {
          inputWrapper: "h-12 min-h-12 rounded-xl",
          input: "text-base",
        },
        lg: {
          inputWrapper: "h-14 min-h-14 rounded-2xl",
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
    ],
  },
  {
    twMerge: true,
  },
);
