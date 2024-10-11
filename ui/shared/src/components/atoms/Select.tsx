import { useButton } from "@react-aria/button";
import { useFocusRing } from "@react-aria/focus";
import { HiddenSelect, useSelect } from "@react-aria/select";
import { mergeProps } from "@react-aria/utils";
import { useSelectState } from "@react-stately/select";
import type { AriaSelectProps } from "@react-types/select";
import { type ReactElement, cloneElement, useMemo, useRef } from "react";
import { useClickAway } from "react-use";

import { type VariantProps, tv } from "tailwind-variants";
import { ArrowSelectorIcon } from "../icons/ArrowSelector";
import { ListBox } from "./Listbox";

export { Item } from "@react-stately/collections";

export interface SelectProps<T extends object>
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
  const {
    selectorIcon: Icon = <ArrowSelectorIcon />,
    placeholder,
    color = "default",
    classNames,
  } = props;
  const state = useSelectState(props);

  const ref = useRef(null);
  const { triggerProps, valueProps, menuProps } = useSelect(props, state, ref);

  const { buttonProps } = useButton(triggerProps, ref);

  const { focusProps } = useFocusRing();

  useClickAway(ref, state.close);

  const { base, listboxWrapper, listbox, value, selectorIcon, trigger } = selectVariants({ color });

  const renderIndicator = cloneElement(Icon as ReactElement, {
    className: `${selectorIcon({ className: classNames?.selectorIcon })}  ${state.isOpen ? "rotate-180" : ""}`,
  });

  const renderSelectedItem = useMemo(() => {
    if (!state.selectedItem) return placeholder;
    return state.selectedItem.rendered;
  }, [state.selectedItem, placeholder]);

  const renderPopover = useMemo(
    () => (
      <div
        className={listboxWrapper({ isOpen: state.isOpen, className: classNames?.listboxWrapper })}
      >
        <ListBox
          {...menuProps}
          state={state}
          className={listbox({ className: classNames?.listbox })}
          color={color}
        />
      </div>
    ),
    [state.isOpen, state, menuProps, color, listboxWrapper, listbox, classNames],
  );

  return (
    <div className={base({ isOpen: state.isOpen, className: classNames?.base })}>
      <HiddenSelect state={state} triggerRef={ref} label={props.label} name={props.name} />
      <button
        {...mergeProps(buttonProps, focusProps)}
        ref={ref}
        className={trigger({ className: classNames?.trigger })}
      >
        <span {...valueProps} className={value({ className: classNames?.value })}>
          {renderSelectedItem}
        </span>
        {renderIndicator}
      </button>
      {renderPopover}
    </div>
  );
}

const selectVariants = tv({
  slots: {
    base: "group inline-flex flex-col relative w-full transition-all shadow-sm",
    listboxWrapper:
      "h-0 py-0 px-4 scroll-py-6 max-h-64 w-full transition-all overflow-hidden absolute",
    listbox: "",
    value: ["text-foreground-500", "font-normal", "w-full", "text-left", "rtl:text-right"],
    selectorIcon: "w-5 h-5 transition-all",
    trigger:
      "relative w-full inline-flex tap-highlight-transparent flex-row items-center px-6 py-3 gap-3 outline-none",
  },
  variants: {
    color: {
      default: {},
      white: {},
    },
    size: {
      sm: {},
      md: {
        base: "rounded-xl",
        trigger: "min-h-12 rounded-xl",
        value: "text-base",
        listboxWrapper: "top-12 rounded-b-xl",
      },
      lg: {
        base: "rounded-2xl",
        trigger: "min-h-14 rounded-2xl",
        value: "text-base",
        listboxWrapper: "top-14 rounded-b-2xl",
      },
    },
    isOpen: {
      true: {
        base: "rounded-t-xl rounded-b-none",
        listboxWrapper: "h-fit z-30 py-4",
      },
    },
  },
  defaultVariants: {
    color: "default",
    size: "md",
  },
  compoundVariants: [
    {
      color: "default",
      class: {
        base: "bg-surface-rose-300 text-typography-rose-600 hover:bg-surface-rose-400",
        listboxWrapper: "bg-surface-rose-300",
      },
    },
    {
      isOpen: true,
      color: "default",
      class: {
        base: "hover:bg-surface-rose-300",
      },
    },
    {
      color: "white",
      class: {
        base: "bg-surface-off-white-200 text-typography-black-200 hover:bg-surface-yellow-100",
        listboxWrapper: "bg-surface-off-white-200",
      },
    },
    {
      isOpen: true,
      color: "white",
      class: {
        base: "hover:bg-surface-off-white-200",
      },
    },
  ],
});
