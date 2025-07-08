import { twMerge } from "#utils/twMerge.js";

import React from "react";

import type { PropsWithChildren } from "react";

interface Props {
  component?: string | React.ReactNode;
  className?: string;
}

export const EmptyPlaceholder: React.FC<PropsWithChildren<Props>> = ({
  children,
  component,
  className,
}) => {
  const hasChildren = React.Children.count(children) > 0;

  const renderedComponent = component ? (
    typeof component === "string" ? (
      <p className="diatype-xs-regular text-gray-700">{component}</p>
    ) : (
      component
    )
  ) : null;

  return (
    <div
      className={twMerge(
        "flex flex-col gap-1 items-center justify-center p-2 w-full bg-[url('./images/notifications/bubble-bg.svg')] bg-[50%_1rem] [background-size:100vw] bg-no-repeat rounded-xl bg-rice-50",
        className,
      )}
    >
      {hasChildren ? children : renderedComponent}
    </div>
  );
};
