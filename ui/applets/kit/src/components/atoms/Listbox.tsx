import type { AriaListBoxOptions } from "@react-aria/listbox";
import { useListBox } from "@react-aria/listbox";
import type { ListState } from "@react-stately/list";
import { useRef } from "react";
import { OptionBox, type OptionBoxProps } from "./OptionBox";

interface ListBoxProps extends AriaListBoxOptions<unknown>, Pick<OptionBoxProps, "color"> {
  listBoxRef?: React.RefObject<HTMLUListElement>;
  state: ListState<unknown>;
  className?: string;
}

export const ListBox: React.FC<ListBoxProps> = (props) => {
  const ref = useRef<HTMLUListElement>(null);
  const { listBoxRef = ref, state, color } = props;
  const { listBoxProps } = useListBox(props, state, listBoxRef);

  return (
    <ul
      {...listBoxProps}
      ref={listBoxRef}
      className="w-full max-h-72 overflow-auto outline-none gap-1 flex flex-col"
    >
      {[...state.collection].map((item) => (
        <OptionBox key={item.key} item={item} state={state} color={color} />
      ))}
    </ul>
  );
};
