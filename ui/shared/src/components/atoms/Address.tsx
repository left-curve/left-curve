import type { ComponentPropsWithoutRef } from "react";
import { twMerge } from "~/utils";

type Props = {
  address?: string;
  children?: string;
};

const Address: React.FC<Props & ComponentPropsWithoutRef<"p">> = ({
  children,
  address,
  className,
  ...props
}) => {
  const addr = children ? children : address ? address : "";
  return (
    <p className={twMerge("flex overflow-auto", className)} {...props}>
      <span className="truncate">{addr.slice(0, -8)}</span>
      <span>{addr.slice(addr.length - 8)}</span>
    </p>
  );
};

export default Address;
