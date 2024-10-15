import type { AriaTabPanelProps } from "@react-aria/tabs";

import { useTabPanel } from "@react-aria/tabs";
import { mergeProps } from "@react-aria/utils";
import type { TabListState } from "@react-stately/tabs";
import type { Key } from "@react-types/shared";
import { forwardRef, useDOMRef } from "~/utils/dom";

import type { As } from "~/types/react";

interface Props {
  as?: As;
  /**
   * The current tab key.
   */
  tabKey: Key;
  /**
   * The tab list state.
   */
  state: TabListState<object>;
}

export type TabPanelProps = Props & AriaTabPanelProps;

const TabPanel = forwardRef<"div", TabPanelProps>((props, ref) => {
  const { as, tabKey, state, className, ...otherProps } = props;

  const domRef = useDOMRef(ref);

  const Component = as || "div";

  const { tabPanelProps } = useTabPanel({ ...props, id: String(tabKey) }, state, domRef);

  const selectedItem = state.selectedItem;

  const content = state.collection.getItem(tabKey)!.props.children;

  const isSelected = tabKey === selectedItem?.key;

  if (!content || !isSelected) {
    return null;
  }

  return (
    <Component
      {...selectedItem.props}
      ref={domRef}
      data-inert={!isSelected ? "true" : undefined}
      inert={!isSelected ? "true" : undefined}
      data-slot="panel"
      {...(isSelected && mergeProps(tabPanelProps, otherProps))}
    >
      {content}
    </Component>
  );
});

export default TabPanel;
