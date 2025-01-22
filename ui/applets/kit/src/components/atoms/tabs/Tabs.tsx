import { useTabList } from "@react-aria/tabs";
import { type TabListStateOptions, useTabListState } from "@react-stately/tabs";
import type React from "react";
import { type ReactElement, useRef } from "react";
import { twMerge } from "../../../utils";
import { Tab } from "./Tab";
import TabPanel from "./TabPanel";

export type TabsProps = TabListStateOptions<{
  id: string;
  title: string;
  content: ReactElement | string;
}> & {
  classNames?: {
    container?: string;
    tabsWrapper?: string;
  };
};

export const Tabs: React.FC<TabsProps> = ({ classNames, ...props }) => {
  const state = useTabListState(props);
  const ref = useRef<HTMLDivElement>(null);
  const { tabListProps } = useTabList(props, state, ref);

  const { container, tabsWrapper } = classNames || {};

  const tabs = [...state.collection].map((item) => (
    <Tab key={item.key} item={item} state={state} {...item.props} />
  ));

  return (
    <div className={twMerge("flex flex-col gap-4 w-full", container)}>
      <div
        ref={ref}
        className={twMerge("flex gap-4 items-center justify-around w-full", tabsWrapper)}
        {...tabListProps}
      >
        {tabs}
      </div>

      {[...state.collection].map((item) => {
        return <TabPanel key={item.key} state={state} tabKey={item.key} />;
      })}
    </div>
  );
};
