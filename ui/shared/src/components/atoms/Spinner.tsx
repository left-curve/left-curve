import type React from "react";
import { forwardRef } from "react";
import { type VariantProps, tv } from "tailwind-variants";
import { twMerge } from "~/utils";

const spinner = tv({
  slots: {
    base: "origin-center",
    circle1: "fill-none stroke-2",
    circle2: "fill-none transition-colors stroke-2",
  },
  variants: {
    size: {
      sm: {
        base: "w-[2rem]",
        circle1: "stroke-[3px]",
        circle2: "stroke-[3px]",
      },
      md: {
        base: "w-[4rem]",
      },
      lg: {
        base: "w-[8rem]",
        circle1: "stroke-1",
        circle2: "stroke-1",
      },
      xl: {
        base: "w-[12rem]",
        circle1: "stroke-1",
        circle2: "stroke-1",
      },
    },
    isLoading: {
      true: {
        base: "animate-rotate-2",
        circle1: "stroke-blue-200",
        circle2:
          "stroke-blue-500 [stroke-dasharray:1,200] [stroke-dashoffset:0] [stroke-linecap:round] animate-dash-4",
      },
    },
    isError: {
      true: {
        circle1: "stroke-red-500",
        circle2:
          "stroke-red-500 [stroke-dasharray:1,200] [stroke-dashoffset:0] [stroke-linecap:round] ",
      },
    },
  },
  defaultVariants: {
    isError: false,
    isLoading: true,
  },
});

export interface SpinnerProps
  extends React.SVGAttributes<SVGSVGElement>,
    VariantProps<typeof spinner> {}

export const Spinner = forwardRef<SVGSVGElement, SpinnerProps>(
  ({ className, isError, isLoading, size, ...props }, ref) => {
    const { base, circle1, circle2 } = spinner({ isError, isLoading, size });
    return (
      <svg ref={ref} {...props} viewBox="25 25 50 50" className={twMerge(base(), className)}>
        <circle r="20" cy="50" cx="50" className={circle1()} />
        <circle r="20" cy="50" cx="50" className={circle2()} />
      </svg>
    );
  },
);

Spinner.displayName = "Spinner";
