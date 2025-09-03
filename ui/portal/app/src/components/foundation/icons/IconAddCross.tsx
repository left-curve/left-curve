import { Path } from "react-native-svg";

import type { SvgProps } from "react-native-svg";
import type React from "react";
import { Svg } from "./SvgBase";

export const IconAddCross: React.FC<SvgProps> = ({ ...props }) => {
  return (
    <Svg width={24} height={24} viewBox="0 0 48 48" fill="none" {...props}>
      <Path
        d="M24 12v24M36 24H12"
        stroke="currentColor"
        strokeWidth={3.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </Svg>
  );
};
