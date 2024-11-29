import { type VariantProps, tv } from "tailwind-variants";
import type { As } from "../../types";
import { forwardRef, twMerge } from "../../utils";
import { Spinner } from "./Spinner";

export interface ButtonProps extends VariantProps<typeof buttonVariants> {
  as?: As;
  /**
   * When true, the button will be disabled.
   * @default false
   */
  isLoading?: boolean;
  isDisabled?: boolean;
}

export const Button = forwardRef<"button", ButtonProps>(
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
      "font-bold",
      "subpixel-antialiased",
      "overflow-hidden",
      "tap-highlight-transparent",
      "data-[pressed=true]:scale-[0.97]",
      "italic",
      "font-exposure",
      "transition-all",
    ],
    variants: {
      variant: {
        solid: "",
        bordered: "border-2 bg-transparent",
        light: "bg-transparent border-none",
      },
      color: {
        none: "",
        gray: "",
        sand: "",
        green: "",
        rose: "",
        purple: "",
      },
      size: {
        sm: "px-12 py-2 text-xs max-h-[2rem]",
        md: "px-12 py-3 max-h-[3rem] text-[20px]",
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
        true: "opacity-disabled pointer-events-none cursor-not-allowed",
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
      {
        variant: "solid",
        color: "gray",
        class:
          "bg-surface-green-200 hover:bg-surface-green-300 text-typography-black-300 not-italic font-diatype-rounded",
      },
      {
        variant: "solid",
        color: "purple",
        class: "bg-surface-purple-200 hover:bg-surface-purple-300 text-typography-purple-400",
      },
      {
        variant: "solid",
        color: "green",
        class: "bg-surface-green-300 hover:bg-surface-green-400 text-typography-green-400",
      },
      {
        variant: "solid",
        color: "sand",
        class: "bg-surface-rose-200 hover:bg-surface-rose-300 text-typography-rose-500",
      },
      // variant / bordered
      {
        variant: "bordered",
        color: "purple",
        class:
          "border-borders-purple-600 bg-surface-purple-100 hover:bg-surface-purple-300 text-typography-purple-400",
      },
      // variant / light
      {
        variant: "light",
        color: "rose",
        class: "text-typography-purple-400 hover:text-typography-purple-500 ",
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
