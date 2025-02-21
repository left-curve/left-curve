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
  base: "rounded-xl py-2 px-3 text-base outline-none cursor-pointer flex items-center transition-all diatype-m-medium",
  variants: {
    color: {
      default: "bg-rice-25 hover:bg-rice-50",
      white: "text-typography-black-200 bg-surface-off-white-200 hover:bg-surface-yellow-200",
    },
  },
  defaultVariants: {
    color: "default",
  },
});
