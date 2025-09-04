import { twMerge } from "@left-curve/foundation";

import { GlobalText } from "./GlobalText";

import type React from "react";
import type { TextProps } from "react-native-svg";

type Props = {
  text?: string;
  className?: string;
  start?: number;
  end?: number;
};

export const TruncateText: React.FC<Props & TextProps> = ({
  text = "",
  className,
  start = 8,
  end = 8,
  ...props
}) => {
  if (text.length <= start + end) {
    return (
      <GlobalText className={twMerge(className)} {...props}>
        {text}
      </GlobalText>
    );
  }

  return (
    <GlobalText className={twMerge("flex-row", className)} {...props}>
      <GlobalText className={twMerge(className)}>{text.slice(0, start)}</GlobalText>
      <GlobalText className={twMerge(className)}>…</GlobalText>
      <GlobalText className={twMerge(className)}>{text.slice(text.length - end)}</GlobalText>
    </GlobalText>
  );
};

export default TruncateText;
