import { tv } from "tailwind-variants";

import type React from "react";
import type { VariantProps } from "tailwind-variants";
import { twMerge } from "@left-curve/foundation";

export interface DotProps extends VariantProps<typeof dotVariants> {
  pulse?: boolean;
}

export const Dot: React.FC<DotProps> = ({ pulse = false, ...rest }) => {
  const styles = dotVariants(rest);
  return (
    <div className={twMerge(styles.root())}>
      {pulse && <span className={twMerge(styles.halo())} />}
      <span className={twMerge(styles.dot())} />
    </div>
  );
};

const dotVariants = tv(
  {
    slots: {
      root: "relative flex items-center justify-center w-4 h-4",
      dot: "relative w-2 h-2 rounded-full",
      halo: "absolute w-4 h-4 rounded-full animate-flash",
    },
    variants: {
      color: {
        success: {
          dot: "bg-utility-success-500",
          halo: "bg-utility-success-100",
        },
        error: {
          dot: "bg-utility-error-500",
          halo: "bg-utility-error-100",
        },
        warning: {
          dot: "bg-utility-warning-500",
          halo: "bg-utility-warning-100",
        },
      },
    },
    defaultVariants: {
      color: "success",
    },
  },
  {
    twMerge: true,
  },
);
