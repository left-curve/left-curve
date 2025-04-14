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
        blue: "bg-blue-100 text-blue-800 border-blue-200",
        red: "bg-red-bean-100 text-red-bean-800 border-red-bean-200",
        green: "bg-green-bean-100 text-green-bean-800 border-green-bean-200",
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
