import { Item } from "@react-stately/collections";
import type { ItemProps } from "@react-types/shared";
import type { ReactNode } from "react";

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
}

export type TabItemProps<T extends object = object> = Props<T>;

export const TabItem = Item as <T extends object>(props: TabItemProps<T>) => JSX.Element;
