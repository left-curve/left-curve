import { tv } from "tailwind-variants";

import type React from "react";
import type { VariantProps } from "tailwind-variants";

export interface BadgeProps extends VariantProps<typeof badgeVariants> {
  text: string;
  className?: string;
}

export const Badge: React.FC<BadgeProps> = ({ text, ...rest }) => {
  return <div className={badgeVariants(rest)}>{text}</div>;
};
const badgeVariants = tv(
  {
    base: ["rounded-[4px] diatype-xs-medium w-fit h-fit"],
    variants: {
      color: {
        blue: "bg-surface-secondary-blue text-foreground-primary-blue border-tertiary-blue",
        red: "bg-surface-secondary-red text-foreground-primary-red border-secondary-red",
        green:
          "bg-surface-tertiary-green text-foreground-primary-green border-surface-primary-green",
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
