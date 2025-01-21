import { useTab } from "@react-aria/tabs";
import type { TabListState } from "@react-stately/tabs";
import type { ItemProps, Node } from "@react-types/shared";

import type { ReactNode } from "react";
import { forwardRef, twMerge, useDOMRef } from "../../../utils";

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
  classNames?: {
    container?: string;
    selected?: string;
  };
}

export const Tab = forwardRef<"button", Props>(({ item, state, classNames }, ref) => {
  const { key, rendered } = item;
  const domRef = useDOMRef(ref);

  const { container, selected } = classNames || {};

  const { tabProps } = useTab(item, state, domRef);

  const isSelected = state.selectedKey === key;

  return (
    <div
      ref={domRef}
      className={twMerge(
        "italic cursor-pointer relative p-1 px-2 focus:outline-none text-typography-green-300",
        "after:content-[''] after:block after:absolute after:inset-0 after:z-0 after:rounded-3xl after:bg-surface-green-100 after:scale-0 after:transition-all",
        { [`after:scale-1 text-typography-green-400 ${selected}`]: isSelected },
        container,
      )}
      {...tabProps}
    >
      <p className={twMerge("relative z-10 px-2 py-1")}>{rendered}</p>
    </div>
  );
});
