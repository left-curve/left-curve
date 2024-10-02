import { useOption } from "@react-aria/listbox";
import type { ListState } from "@react-stately/list";
import type { Node } from "@react-types/shared";
import { useRef } from "react";
import { type VariantProps, tv } from "tailwind-variants";

export interface OptionBoxProps extends VariantProps<typeof optionVariants> {
  item: Node<unknown>;
  state: ListState<unknown>;
}

export const OptionBox: React.FC<OptionBoxProps> = ({ item, state, color }) => {
  const ref = useRef<HTMLLIElement>(null);
  const { optionProps } = useOption({ key: item.key }, state, ref);

  const styles = optionVariants({ color });

  return (
    <li {...optionProps} ref={ref} className={styles}>
      {item.rendered}
    </li>
  );
};

const optionVariants = tv({
  base: "rounded-xl py-2 px-4 text-base outline-none cursor-pointer flex items-center justify-between",
  variants: {
    color: {
      default: "text-typography-rose-600 bg-surface-rose-300 hover:bg-surface-rose-400",
    },
  },
  defaultVariants: {
    color: "default",
  },
});
