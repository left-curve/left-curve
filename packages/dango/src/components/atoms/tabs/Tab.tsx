import { twMerge } from "~/utils";

import { useTab } from "@react-aria/tabs";
import type { TabListState } from "@react-stately/tabs";
import type { ItemProps, Node } from "@react-types/shared";
import { AnimatePresence, motion } from "framer-motion";

import type { ReactNode } from "react";
import { forwardRef, useDOMRef } from "~/utils/dom";

interface Props<T extends object = object> extends Omit<ItemProps<T>, "children" | "title"> {
  /**
   * The content of the component.
   */
  children?: ReactNode | null;
  /**
   * The title of the component.
   */
  title?: ReactNode | null;
  /**
   *  A string representation of the item's contents. Use this when the title is not readable.
   *  This will be used as native `title` attribute.
   * */
  titleValue?: string;
  /** Whether the tab should be disabled. */
  isDisabled?: boolean;

  item: Node<T>;
  state: TabListState<T>;
}

export const Tab = forwardRef<"button", Props>(({ item, state }, ref) => {
  const { key, rendered } = item;
  const domRef = useDOMRef(ref);

  const { tabProps } = useTab(item, state, domRef);

  const isSelected = state.selectedKey === key;

  return (
    <div
      ref={domRef}
      className={twMerge(
        "italic cursor-pointer relative p-1 px-2 focus:outline-none",
        isSelected ? "text-typography-green-400" : "text-typography-green-300",
      )}
      {...tabProps}
    >
      <AnimatePresence>
        {isSelected && (
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            exit={{ scale: 0 }}
            className="absolute top-0 left-0 rounded-3xl bg-surface-green-100 h-full w-full"
          />
        )}
      </AnimatePresence>
      <p className="relative z-10 px-2 py-1">{rendered}</p>
    </div>
  );
});
