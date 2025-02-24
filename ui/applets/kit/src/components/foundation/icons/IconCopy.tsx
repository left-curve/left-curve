import type React from "react";
import { useState } from "react";
import { IconCopyCheck } from "./IconCopyCheck";
import { IconCopyNoCheck } from "./IconCopyNoCheck";

export const IconCopy: React.FC<React.SVGAttributes<HTMLOrSVGElement> & { copyText?: string }> = ({
  copyText,
  ...props
}) => {
  const [copyIcon, setCopyIcon] = useState(<IconCopyNoCheck {...props} />);

  return (
    <button
      type="button"
      onClick={() => {
        if (copyText) navigator.clipboard.writeText(copyText);
        setCopyIcon(<IconCopyCheck {...props} />);
        setTimeout(() => setCopyIcon(<IconCopyNoCheck {...props} />), 1000);
      }}
    >
      {copyIcon}
    </button>
  );
};
