import { HiddenSelect, useSelect } from "@react-aria/select";
import { useSelectState } from "@react-stately/select";
import type { AriaSelectProps } from "@react-types/select";
import { AnimatePresence, motion } from "framer-motion";
import { type ReactElement, cloneElement, useMemo, useRef } from "react";
import { useClickAway } from "react-use";

import { type VariantProps, tv } from "tailwind-variants";
import { twMerge } from "../../utils";
import { ListBox } from "./Listbox";
import { IconChevronDown } from "./icons/IconChevronDown";

export { Item } from "@react-stately/collections";

export interface SelectProps<T extends object = object>
  extends AriaSelectProps<T>,
    VariantProps<typeof selectVariants> {
  selectorIcon?: React.ReactNode;
  endContent?: React.ReactNode;
  bottomComponent?: React.ReactNode;
  error?: string;
  classNames?: {
    base?: string;
    listboxWrapper?: string;
    listbox?: string;
    value?: string;
    selectorIcon?: string;
    trigger?: string;
  };
}

export function Select<T extends object>(props: SelectProps<T>) {
  const { selectorIcon: Icon = <IconChevronDown />, placeholder, classNames } = props;
  const state = useSelectState(props);

  const ref = useRef(null);
  const { valueProps, menuProps } = useSelect(props, state, ref);

  useClickAway(ref, state.close);

  const { base, listboxWrapper, selectorIcon, trigger } = selectVariants();

  const renderIndicator = cloneElement(Icon as ReactElement, {
    className: `${selectorIcon({ className: classNames?.selectorIcon })}  ${state.isOpen ? "rotate-180" : ""}`,
  });

  const renderSelectedItem = useMemo(() => {
    if (!state.selectedItem) return placeholder;
    return state.selectedItem.rendered;
  }, [state.selectedItem, placeholder]);

  return (
    <div className={base({ className: classNames?.base })}>
      <HiddenSelect state={state} triggerRef={ref} label={props.label} name={props.name} />
      <button
        ref={ref}
        type="button"
        onClick={() => state.setOpen(!state.isOpen)}
        className={trigger({ className: classNames?.trigger })}
      >
        <span {...valueProps}>{renderSelectedItem}</span>
        {renderIndicator}
      </button>
      <motion.div layout className="overflow-hidden">
        <AnimatePresence mode="wait">
          <motion.div
            className={twMerge(
              listboxWrapper({
                className: classNames?.listboxWrapper,
              }),
              { hidden: !state.isOpen, block: state.isOpen },
            )}
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0, padding: 0 }}
            transition={{ duration: 0.1 }}
          >
            <ListBox {...menuProps} state={state} />
          </motion.div>
        </AnimatePresence>
      </motion.div>
    </div>
  );
}

const selectVariants = tv({
  slots: {
    base: "group inline-flex flex-col relative w-fit min-w-[9rem] transition-all  duration-500 leading-none",
    listboxWrapper:
      "rounded-md overflow-hidden max-h-[12rem] w-full transition-all z-50 shadow-card-shadow top-[3.375rem] bg-rice-25 absolute",
    selectorIcon: "min-w-[20px] min-h-[20px] transition-all duration-300",
    trigger:
      "w-full inline-flex tap-highlight-transparent flex-row items-center justify-between px-4 py-3 gap-3 outline-none shadow-card-shadow diatype-m-regular h-[46px] rounded-md bg-rice-25",
  },
  variants: {},
});
