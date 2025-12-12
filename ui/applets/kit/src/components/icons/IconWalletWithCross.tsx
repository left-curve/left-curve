import type React from "react";
import { IconClose } from "./IconClose";
import { IconWallet } from "./IconWallet";
import { twMerge } from "@left-curve/foundation";

interface IconWalletWithCrossProps extends React.SVGAttributes<HTMLOrSVGElement> {
  isCrossVisible: boolean;
}

export const IconWalletWithCross: React.FC<IconWalletWithCrossProps> = ({
  isCrossVisible,
  ...props
}) => {
  return (
    <div className="relative">
      {isCrossVisible && (
        <div className="absolute right-[-4px] top-[-4px] rounded-full h-4 w-4 bg-ink-placeholder-400 flex items-center justify-center border-2 border-surface-quaternary-rice ">
          <IconClose className="w-full h-full text-surface-primary-rice" />
        </div>
      )}
      <IconWallet
        className={twMerge("w-6 h-6", { "text-ink-placeholder-400": isCrossVisible })}
        {...props}
      />
    </div>
  );
};
