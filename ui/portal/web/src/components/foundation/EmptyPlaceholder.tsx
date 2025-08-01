import React from "react";

import { twMerge } from "@left-curve/applets-kit";

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
      <p className="diatype-xs-regular text-secondary-700">{component}</p>
    ) : (
      component
    )
  ) : null;

  return (
    <div
      className={twMerge("flex flex-col gap-1 items-center justify-center p-2 w-full", className)}
    >
      {hasChildren ? children : renderedComponent}
    </div>
  );
};
