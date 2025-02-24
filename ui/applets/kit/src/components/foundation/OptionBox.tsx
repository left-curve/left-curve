import { useOption } from "@react-aria/listbox";
import type { ListState } from "@react-stately/list";
import type { Node } from "@react-types/shared";
import { motion } from "framer-motion";
import { useRef } from "react";
import { type VariantProps, tv } from "tailwind-variants";
export interface OptionBoxProps extends VariantProps<typeof optionVariants> {
  item: Node<unknown>;
  state: ListState<unknown>;
}

const childVariants = {
  hidden: { opacity: 0, y: -10 },
  visible: { opacity: 1, y: 0 },
};

export const OptionBox: React.FC<OptionBoxProps> = ({ item, state, color }) => {
  const ref = useRef<HTMLLIElement>(null);
  const { optionProps } = useOption({ key: item.key }, state, ref);

  const styles = optionVariants({ color });

  return (
    <motion.li variants={childVariants}>
      <span {...optionProps} ref={ref} className={styles}>
        {item.rendered}
      </span>
    </motion.li>
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
