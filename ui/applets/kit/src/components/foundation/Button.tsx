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
    { as, fullWidth, variant, size, radius, isDisabled, isLoading, className, children, ...props },
    ref,
  ) => {
    const Component = as ?? "button";
    const styles = buttonVariants({
      variant,
      size,
      radius,
      fullWidth,
      isDisabled,
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
      "cursor-pointer",
      "items-center",
      "justify-center",
      "box-border",
      "appearance-none",
      "outline-none",
      "select-none",
      "whitespace-nowrap",
      "min-w-max",
      "subpixel-antialiased",
      "overflow-hidden",
      "tap-highlight-transparent",
      "data-[pressed=true]:scale-[0.97]",
      "italic",
      "transition-all",
    ],
    variants: {
      variant: {
        primary:
          "rounded-full shadow-btn-shadow-gradient transition-all duration-300 flex items-center justify-center w-fit",
        secondary:
          "rounded-full shadow-btn-shadow-gradient transition-all duration-300 flex items-center justify-center w-fit",
        utility: " shadow-btn-shadow-gradient transition-all duration-300 w-fit",
        link: "rounded-xl transition-all duration-300 w-fit mx-1",
      },
      size: {
        xs: "h-[25px] py-1 px-[6px] exposure-xs-italic text-xs gap-[2px]",
        sm: "h-[32px] py-[6px] px-2 exposure-sm-italic gap-[2px]",
        md: "h-[40px] py-[10px] px-3 exposure-sm-italic text-md gap-[4px]",
        lg: "h-[44px] py-[11px] px-3 exposure-m-italic text-lg gap-[4px]",
        xl: "h-[56px] py-[14px] px-4 exposure-l-italic text-h4 gap-[6px]",
      },
      radius: {
        none: "rounded-none",
        sm: "rounded-sm",
        md: "rounded-md",
        lg: "rounded-lg",
        xl: "rounded-xl",
        full: "rounded-full",
      },
      fullWidth: {
        true: "w-full",
      },
      isDisabled: {
        true: "pointer-events-none cursor-not-allowed",
      },
    },
    defaultVariants: {
      size: "md",
      variant: "primary",
      fullWidth: false,
      isDisabled: false,
    },
    compoundVariants: [
      {
        variant: "primary",
        class:
          "bg-red-bean-400 hover:bg-red-bean-600 text-white-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(0,_0,_0,_0.04)_8%,_rgba(0,_0,_0,_0.07)_100%)]",
      },
      {
        variant: "secondary",
        class:
          "bg-blue-50 hover:bg-blue-100 text-blue-500 focus:[box-shadow:0px_0px_0px_3px_#E2E3F2] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(0,_0,_0,_0.04)_8%,_rgba(0,_0,_0,_0.07)_100%)]",
      },
      {
        variant: "link",
        class:
          "bg-transparent hover:text-blue-600 text-blue-500 focus:bg-blue-50 focus:[box-shadow:0px_0px_0px_3px_#F0F1F7]",
      },
      {
        variant: "link",
        isDisabled: true,
        class: "text-gray-200",
      },
      {
        variant: "primary",
        isDisabled: true,
        class: "bg-gray-50 text-gray-200 shadow-btn-shadow-disabled ",
      },
      {
        variant: "secondary",
        isDisabled: true,
        class:
          "bg-blue-50 text-blue-200 shadow-btn-shadow-disabled border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(0,_0,_0,_0.04)_8%,_rgba(0,_0,_0,_0.07)_100%)]",
      },
      {
        variant: "utility",
        class:
          "bg-rice-100 hover:bg-rice-200 text-rice-700 focus:[box-shadow:0px_0px_0px_3px_#FFF3E1B3] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)]",
      },
      {
        variant: "utility",
        isDisabled: true,
        class:
          "bg-gray-50 text-gray-200 shadow-btn-shadow-disabled border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.06)_8%,_rgba(46,_37,_33,_0.12)_100%)]",
      },
      {
        variant: ["utility", "link"],
        size: "xs",
        class: "rounded-xs",
      },
      {
        variant: ["utility", "link"],
        size: "sm",
        class: "rounded-sm",
      },
      {
        variant: ["utility", "link"],
        size: "md",
        class: "rounded-md",
      },
      {
        variant: ["utility", "link"],
        size: "lg",
        class: "rounded-lg",
      },
      {
        variant: ["utility", "link"],
        size: "xl",
        class: "rounded-xl",
      },
    ],
  },
  {
    twMerge: true,
  },
);
