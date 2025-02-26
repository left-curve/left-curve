import type { AriaListBoxOptions } from "@react-aria/listbox";
import { useListBox } from "@react-aria/listbox";
import type { ListState } from "@react-stately/list";
import { motion } from "framer-motion";
import { useRef } from "react";
import { twMerge } from "../../utils";
import { OptionBox, type OptionBoxProps } from "./OptionBox";

interface ListBoxProps extends AriaListBoxOptions<unknown>, Pick<OptionBoxProps, "color"> {
  listBoxRef?: React.RefObject<HTMLUListElement>;
  state: ListState<unknown>;
  className?: string;
}

const containerVariants = {
  hidden: {},
  visible: {
    transition: {
      delayChildren: 0.1,
      staggerChildren: 0.1,
    },
  },
};

const childVariants = {
  hidden: { opacity: 0, y: -10 },
  visible: { opacity: 1, y: 0 },
};

export const ListBox: React.FC<ListBoxProps> = (props) => {
  const ref = useRef<HTMLUListElement>(null);
  const { listBoxRef = ref, state, color } = props;
  const { listBoxProps } = useListBox(props, state, listBoxRef);

  return (
    <motion.ul
      ref={listBoxRef}
      className="w-full max-h-[12rem] pb-4 outline-none gap-1 flex flex-col overflow-auto scrollbar-none"
      variants={containerVariants}
      initial="hidden"
      animate="visible"
    >
      {[...state.collection].map((item) => (
        <motion.li variants={childVariants} key={item.key}>
          <span className="rounded-md py-2 px-3 text-base outline-none cursor-pointer flex items-center transition-all diatype-m-medium bg-rice-25 hover:bg-rice-50 leading-none">
            {item.rendered}
          </span>
        </motion.li>
      ))}
    </motion.ul>
  );
};
