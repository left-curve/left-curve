import { type VariantProps, tv } from "tailwind-variants";

export type SpinnerProps = {
  className?: string;
  fullContainer?: boolean;
} & SpinnerVariantProps;

const Spinner: React.FC<SpinnerProps> = ({ className, color, size, fullContainer }) => {
  const { base, wrapper, circle1, circle2 } = styles();

  const spinner = (
    <div className={base({ color, size })}>
      <div className={wrapper({ color, size, className })}>
        <i className={circle1({ color, size })} />
        <i className={circle2({ color, size })} />
      </div>
    </div>
  );

  if (!fullContainer) return spinner;

  return <div className="w-full h-full flex items-center justify-center">{spinner}</div>;
};

export type SpinnerVariantProps = VariantProps<typeof styles>;
export type SpinnerSlots = keyof ReturnType<typeof styles>;

export { Spinner };

const styles = tv({
  slots: {
    base: "relative inline-flex flex-col gap-2 items-center justify-center",
    circle1: [
      "absolute",
      "w-full",
      "h-full",
      "rounded-full",
      "animate-spinner-ease-spin",
      "border-2",
      "border-solid",
      "border-t-transparent",
      "border-l-transparent",
      "border-r-transparent",
    ],
    circle2: [
      "absolute",
      "w-full",
      "h-full",
      "rounded-full",
      "opacity-75",
      "animate-spinner-linear-spin",
      "border-2",
      "border-dotted",
      "border-t-transparent",
      "border-l-transparent",
      "border-r-transparent",
    ],
    wrapper: "relative flex",
  },
  variants: {
    size: {
      xs: {
        wrapper: "w-4 h-4",
        circle1: "border-2",
        circle2: "border-2",
      },
      sm: {
        wrapper: "w-5 h-5",
        circle1: "border-2",
        circle2: "border-2",
      },
      md: {
        wrapper: "w-6 h-6",
        circle1: "border-[3px]",
        circle2: "border-[3px]",
      },
      lg: {
        wrapper: "w-8 h-8",
        circle1: "border-[3px]",
        circle2: "border-[3px]",
      },
      xl: {
        wrapper: "w-10 h-10",
        circle1: "border-[3px]",
        circle2: "border-[3px]",
      },
    },
    color: {
      current: {
        circle1: "border-b-current",
        circle2: "border-b-current",
      },
      gray: {
        circle1: "border-b-gray-500",
        circle2: "border-b-gray-500",
      },
      white: {
        circle1: "border-b-white",
        circle2: "border-b-white",
      },
      green: {
        circle1: "border-b-green-bean-300",
        circle2: "border-b-green-bean-300",
      },
      pink: {
        circle1: "border-b-red-bean-300",
        circle2: "border-b-red-bean-300",
      },
      blue: {
        circle1: "border-b-blue-300",
        circle2: "border-b-blue-500",
      },
    },
  },
  defaultVariants: {
    size: "sm",
    color: "white",
  },
});
