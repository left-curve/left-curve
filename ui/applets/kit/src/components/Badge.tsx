import { tv } from "tailwind-variants";

import type React from "react";
import type { VariantProps } from "tailwind-variants";
import { twMerge } from "@left-curve/foundation";

export interface BadgeProps extends VariantProps<typeof badgeVariants> {
  text: string | React.ReactNode;
  className?: string;
}

export const Badge: React.FC<BadgeProps> = ({ text, className, ...rest }) => {
  return <div className={twMerge(badgeVariants(rest), className)}>{text}</div>;
};
const badgeVariants = tv(
  {
    base: ["rounded-[4px] diatype-xs-medium w-fit h-fit"],
    variants: {
      color: {
        blue: "bg-surface-secondary-blue text-fg-primary-blue border-outline-tertiary-blue",
        red: "bg-surface-secondary-red text-fg-primary-red border-outline-secondary-red",
        green: "bg-surface-tertiary-green text-fg-primary-green border-outline-primary-green",
        rice: "bg-surface-quaternary-rice text-fg-primary-rice border-outline-secondary-rice",
        "light-red": "bg-utility-error-25 text-utility-error-600 border-utility-error-100",
        "light-green": "bg-utility-success-25 text-utility-success-500 border-utility-success-50",
        gray: "bg-utility-gray-100 text-utility-gray-500 border-utility-gray-200",
        warning: "bg-utility-warning-50 text-utility-warning-700 border-utility-warning-100",
      },
      size: {
        s: "py-[2px] px-2 diatype-xs-medium",
        m: "py-[3px] px-1 border diatype-xs-medium",
      },
    },
    defaultVariants: {
      color: "blue",
      size: "m",
    },
  },
  {
    twMerge: true,
  },
);
