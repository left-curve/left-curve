import type { AriaListBoxOptions } from "@react-aria/listbox";
import { useListBox } from "@react-aria/listbox";
import type { ListState } from "@react-stately/list";
import { motion } from "framer-motion";
import { useRef } from "react";
import { OptionBox } from "./OptionBox";

interface ListBoxProps extends AriaListBoxOptions<unknown> {
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

export const ListBox: React.FC<ListBoxProps> = (props) => {
  const ref = useRef<HTMLUListElement>(null);
  const { listBoxRef = ref, state } = props;
  const { listBoxProps } = useListBox(props, state, listBoxRef);

  return (
    <motion.ul
      ref={listBoxRef}
      className="w-full max-h-[12rem] p-2 outline-none gap-1 flex flex-col overflow-auto scrollbar-none"
      variants={containerVariants}
      initial="hidden"
      animate="visible"
    >
      {[...state.collection].map((item) => (
        <OptionBox key={item.key} item={item} state={state} />
      ))}
    </motion.ul>
  );
};
