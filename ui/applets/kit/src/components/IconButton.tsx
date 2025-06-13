import { type VariantProps, tv } from "tailwind-variants";
import { forwardRefPolymorphic, twMerge } from "#utils/index.js";
import { Spinner } from "./Spinner";

export interface IconButtonProps extends VariantProps<typeof buttonVariants> {
  /**
   * When true, the button will be disabled.
   * @default false
   */
  isLoading?: boolean;
  isDisabled?: boolean;
}

export const IconButton = forwardRefPolymorphic<"button", IconButtonProps>(
  (
    {
      as,
      fullWidth,
      variant,
      size,
      radius,
      isDisabled,
      isLoading,
      className,
      children,
      color,
      ...props
    },
    ref,
  ) => {
    const Component = as ?? "button";
    const styles = buttonVariants({
      variant,
      size,
      radius,
      fullWidth,
      isDisabled,
      color,
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
      "cursor-pointer",
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
          "rounded-full shadow-btn-shadow-gradient transition-all duration-300 flex items-center justify-center w-fit bg-red-bean-400 hover:bg-red-bean-600 text-white-100",
        secondary:
          "rounded-full shadow-btn-shadow-gradient transition-all duration-300 flex items-center justify-center w-fit",
        utility:
          "shadow-btn-shadow-gradient transition-all duration-300 w-fit bg-rice-100 hover:bg-rice-200 text-rice-700",
        link: "rounded-xl transition-all duration-300 w-fit bg-transparent hover:text-gray-600 text-tertiary-500",
      },
      color: {
        blue: "",
        red: "",
        green: "",
      },
      size: {
        xs: "p-[4px] h-[24px] w-[24px]",
        sm: "p-[7px] h-[32px] w-[32px]",
        md: "p-[9px] h-[40px] w-[40px]",
        lg: "p-[10px] h-[44px] w-[44px]",
        xl: "p-[14px] h-[56px] w-[56px] rounded-md",
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
      color: "blue",
      fullWidth: false,
      isDisabled: false,
    },
    compoundVariants: [
      {
        variant: "secondary",
        color: "blue",
        class: "bg-blue-50 hover:bg-blue-100 text-blue-500",
      },
      {
        variant: "secondary",
        color: "red",
        class: "bg-primary-red hover:bg-red-bean-100 text-red-bean-500",
      },
      {
        variant: "secondary",
        color: "green",
        class: "bg-green-bean-50 hover:bg-green-bean-100 text-green-bean-500",
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
          "bg-gray-50 text-gray-200 shadow-btn-shadow-disabled  border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(0,_0,_0,_0.04)_8%,_rgba(0,_0,_0,_0.07)_100%)]",
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
