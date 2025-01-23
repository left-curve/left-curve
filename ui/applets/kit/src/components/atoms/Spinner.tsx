import { type VariantProps, tv } from "tailwind-variants";

const Spinner: React.FC<SpinnerVariantProps> = (props) => {
  const { base, wrapper, circle1, circle2 } = spinner();
  return (
    <div className={base()}>
      <div className={wrapper(props)}>
        <i className={circle1(props)} />
        <i className={circle2(props)} />
      </div>
    </div>
  );
};

export type SpinnerVariantProps = VariantProps<typeof spinner>;
export type SpinnerSlots = keyof ReturnType<typeof spinner>;

export { Spinner };

const spinner = tv({
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
      sm: {
        wrapper: "w-5 h-5",
        circle1: "border-2",
        circle2: "border-2",
      },
      md: {
        wrapper: "w-8 h-8",
        circle1: "border-[3px]",
        circle2: "border-[3px]",
      },
      lg: {
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
      white: {
        circle1: "border-b-white",
        circle2: "border-b-white",
      },
      green: {
        circle1: "border-b-brand-green",
        circle2: "border-b-brand-green",
      },
      pink: {
        circle1: "border-b-surface-pink-300",
        circle2: "border-b-surface-pink-300",
      },
    },
  },
  defaultVariants: {
    size: "sm",
    color: "white",
  },
});
