import { twMerge } from "@left-curve/foundation";
import { Button, type ButtonProps } from "./Button";

import type React from "react";

type LinkProps = React.AnchorHTMLAttributes<HTMLAnchorElement> & {
  size?: ButtonProps["size"];
  className?: string;
};

export const Link: React.FC<LinkProps> = ({ size = "xs", className, children, ...props }) => {
  return (
    <Button
      as="a"
      variant="link"
      size={size}
      className={twMerge("!p-0 !h-auto", className)}
      {...props}
    >
      {children}
    </Button>
  );
};
