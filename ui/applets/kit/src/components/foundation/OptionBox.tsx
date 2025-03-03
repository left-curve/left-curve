import { useOption } from "@react-aria/listbox";
import type { ListState } from "@react-stately/list";
import type { Node } from "@react-types/shared";
import { motion } from "framer-motion";
import { useRef } from "react";
export interface OptionBoxProps {
  item: Node<unknown>;
  state: ListState<unknown>;
}

const childVariants = {
  hidden: { opacity: 0, y: -10 },
  visible: { opacity: 1, y: 0 },
};

export const OptionBox: React.FC<OptionBoxProps> = ({ item, state }) => {
  const ref = useRef<HTMLLIElement>(null);
  const { optionProps } = useOption({ key: item.key }, state, ref);

  return (
    <motion.li variants={childVariants}>
      <span
        {...optionProps}
        ref={ref}
        className="rounded-sm py-2 px-3 text-base outline-none cursor-pointer flex items-center transition-all diatype-m-medium bg-rice-25 hover:bg-rice-50 leading-none"
      >
        {item.rendered}
      </span>
    </motion.li>
  );
};
