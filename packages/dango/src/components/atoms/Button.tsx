import { forwardRef } from "react";
import { twMerge } from "~/utils";

import { Slot } from "@radix-ui/react-slot";

import { colorVariants } from "@leftcurve/config/tailwind/colorVariants";
import { type VariantProps, tv } from "tailwind-variants";

export interface ButtonProps
  extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "color">,
    VariantProps<typeof buttonVariants> {
  /**
   * When true, the button will render as a `Slot` component.
   * @default false
   */
  asChild?: boolean;
  /**
   * When true, the button will be disabled.
   * @default false
   */
  isDisabled?: boolean;
}

const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return <Comp className={twMerge(buttonVariants(props), className)} ref={ref} {...props} />;
  },
);

Button.displayName = "Button";

export { Button, buttonVariants };

const buttonVariants = tv({
  base: "inline-flex items-center justify-center whitespace-nowrap rounded-2xl text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 font-bold font-grotesk",
  variants: {
    variant: {
      solid: "hover:brightness-95",
      outline: "border bg-transparent",
      light: "bg-transparent",
      flat: "",
      faded: "border",
      shadow: "",
      dark: "",
      ghost: "border bg-transparent",
    },
    color: {
      default: "",
      white: "",
      purple: "",
      green: "",
      danger: "",
      sand: "",
    },
    size: {
      default: "h-10 px-4 py-2",
      sm: "h-9 px-3",
      lg: "h-11 px-8",
      icon: "h-6 w-6",
      none: "h-fit w-fit p-0",
    },
  },
  defaultVariants: {
    variant: "solid",
    color: "default",
    size: "default",
  },
  compoundVariants: [
    // solid / color
    {
      variant: "solid",
      color: "default",
      class: colorVariants.solid.default,
    },
    {
      variant: "solid",
      color: "white",
      class: colorVariants.solid.default,
    },
    {
      variant: "solid",
      color: "purple",
      class: colorVariants.solid.purple,
    },
    {
      variant: "solid",
      color: "green",
      class: colorVariants.solid.green,
    },
    {
      variant: "solid",
      color: "danger",
      class: colorVariants.solid.danger,
    },
    {
      variant: "solid",
      color: "sand",
      class: colorVariants.solid.sand,
    },
    // shadow / color
    {
      variant: "shadow",
      color: "default",
      class: colorVariants.shadow.default,
    },
    {
      variant: "shadow",
      color: "purple",
      class: colorVariants.shadow.purple,
    },
    {
      variant: "shadow",
      color: "green",
      class: colorVariants.shadow.green,
    },
    {
      variant: "shadow",
      color: "danger",
      class: colorVariants.shadow.danger,
    },
    {
      variant: "shadow",
      color: "sand",
      class: colorVariants.shadow.sand,
    },
    // outline / color
    {
      variant: "outline",
      color: "default",
      class: colorVariants.outline.default,
    },
    {
      variant: "outline",
      color: "purple",
      class: colorVariants.outline.purple,
    },
    {
      variant: "outline",
      color: "green",
      class: colorVariants.outline.green,
    },
    {
      variant: "outline",
      color: "danger",
      class: colorVariants.outline.danger,
    },
    {
      variant: "outline",
      color: "sand",
      class: colorVariants.outline.sand,
    },
    // flat / color
    {
      variant: "flat",
      color: "default",
      class: colorVariants.flat.default,
    },
    {
      variant: "flat",
      color: "purple",
      class: colorVariants.flat.purple,
    },
    {
      variant: "flat",
      color: "green",
      class: colorVariants.flat.green,
    },
    {
      variant: "flat",
      color: "danger",
      class: colorVariants.flat.danger,
    },
    {
      variant: "flat",
      color: "sand",
      class: colorVariants.flat.sand,
    },
    // faded / color
    {
      variant: "faded",
      color: "default",
      class: colorVariants.faded.default,
    },
    {
      variant: "faded",
      color: "purple",
      class: colorVariants.faded.purple,
    },
    {
      variant: "faded",
      color: "green",
      class: colorVariants.faded.green,
    },
    {
      variant: "faded",
      color: "danger",
      class: colorVariants.faded.danger,
    },
    {
      variant: "faded",
      color: "sand",
      class: colorVariants.faded.sand,
    },
    // light / color
    {
      variant: "light",
      color: "default",
      class: colorVariants.light.default,
    },
    {
      variant: "light",
      color: "purple",
      class: colorVariants.light.purple,
    },
    {
      variant: "light",
      color: "green",
      class: colorVariants.light.green,
    },
    {
      variant: "light",
      color: "danger",
      class: colorVariants.light.danger,
    },
    {
      variant: "light",
      color: "sand",
      class: colorVariants.light.sand,
    },
    // dark / color
    {
      variant: "dark",
      color: "default",
      class: colorVariants.dark.default,
    },
    {
      variant: "dark",
      color: "purple",
      class: colorVariants.dark.purple,
    },
    {
      variant: "dark",
      color: "green",
      class: colorVariants.dark.green,
    },
    {
      variant: "dark",
      color: "danger",
      class: colorVariants.dark.danger,
    },
    {
      variant: "dark",
      color: "sand",
      class: colorVariants.dark.sand,
    },
    // ghost / color
    {
      variant: "ghost",
      color: "default",
      class: colorVariants.ghost.default,
    },
    {
      variant: "ghost",
      color: "purple",
      class: colorVariants.ghost.purple,
    },
    {
      variant: "ghost",
      color: "green",
      class: colorVariants.ghost.green,
    },
    {
      variant: "ghost",
      color: "danger",
      class: colorVariants.ghost.danger,
    },
    {
      variant: "ghost",
      color: "sand",
      class: colorVariants.ghost.sand,
    },
  ],
});
