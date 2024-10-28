import { type VariantProps, tv } from "tailwind-variants";
import type { As } from "../../types";
import { forwardRef, twMerge } from "../../utils";
import { Spinner } from "./Spinner";

export interface ButtonProps
  extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "color">,
    VariantProps<typeof buttonVariants> {
  as?: As;
  /**
   * When true, the button will be disabled.
   * @default false
   */
  isLoading?: boolean;
  isDisabled?: boolean;
}

export const DangoButton = forwardRef<"button", ButtonProps>(
  (
    {
      as,
      fullWidth,
      variant,
      color,
      size,
      radius,
      isInGroup,
      isDisabled,
      isLoading,
      isIconOnly,
      className,
      children,
      ...props
    },
    ref,
  ) => {
    const Component = as ?? "button";
    const styles = buttonVariants({
      variant,
      color,
      size,
      radius,
      fullWidth,
      isDisabled,
      isInGroup,
      isIconOnly,
    });

    const disabled = isDisabled || isLoading;

    return (
      <Component className={twMerge(styles, className)} ref={ref} {...props} disabled={disabled}>
        {isLoading ? <Spinner size={size} /> : children}
      </Component>
    );
  },
);

const buttonVariants = tv(
  {
    base: [
      "z-0",
      "group",
      "relative",
      "inline-flex",
      "items-center",
      "justify-center",
      "box-border",
      "appearance-none",
      "outline-none",
      "select-none",
      "whitespace-nowrap",
      "min-w-max",
      "font-normal",
      "subpixel-antialiased",
      "overflow-hidden",
      "tap-highlight-transparent",
      "data-[pressed=true]:scale-[0.97]",
    ],
    variants: {
      variant: {
        solid: "",
        bordered: "border-2 bg-transparent",
        ghost: "bg-transparent border-none",
      },
      color: {
        rose: "",
        sand: "",
      },
      size: {
        sm: "px-12 py-2 text-xs",
        md: "px-12 py-3",
      },
      radius: {
        none: "rounded-none",
        sm: "rounded-small",
        md: "rounded-medium",
        lg: "rounded-large",
        xl: "rounded-[48px]",
        full: "rounded-full",
      },
      fullWidth: {
        true: "w-full",
      },
      isDisabled: {
        true: "opacity-disabled pointer-events-none",
      },
      isInGroup: {
        true: "[&:not(:first-child):not(:last-child)]:rounded-none",
      },
      isIconOnly: {
        true: "px-0 !gap-0",
        false: "[&>svg]:max-w-[theme(spacing.8)]",
      },
    },
    defaultVariants: {
      size: "md",
      radius: "xl",
      color: "rose",
      variant: "solid",
      fullWidth: false,
      isDisabled: false,
      isInGroup: false,
    },
    compoundVariants: [
      // variant / solid
      {
        variant: "solid",
        color: "rose",
        class: "bg-surface-pink-200 hover:bg-surface-pink-300 text-surface-rose-200",
      },
      // variant / bordered
      {
        variant: "bordered",
        color: "sand",
        class: "border-typography-rose-600 hover:bg-surface-rose-600/20 text-typography-rose-600",
      },
      // variant / ghost
      {
        variant: "ghost",
        color: "sand",
        class: "text-typography-rose-500 hover:text-typography-rose-600",
      },
      // variant / iconOnly
      {
        isIconOnly: true,
        size: "sm",
        class: "min-w-8 w-8 h-8",
      },
      {
        isIconOnly: true,
        size: "md",
        class: "min-w-10 w-10 h-10",
      },
      // variant / hover
      {
        variant: ["solid", "bordered"],
        class: "data-[hover=true]:opacity-hover",
      },
    ],
  },
  {
    twMerge: true,
  },
);
