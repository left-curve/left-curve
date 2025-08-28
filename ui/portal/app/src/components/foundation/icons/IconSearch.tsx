import { Path } from "react-native-svg";
import { Svg } from "./SvgBase";

import type React from "react";
import type { SvgProps } from "react-native-svg";

export const IconSearch: React.FC<SvgProps> = ({ ...props }) => {
  return (
    <Svg width={20} height={20} viewBox="0 0 20 20" fill="none" {...props}>
      <Path
        d="m16.666 16.668-2.41-2.41M9.6 15.868c4.011 0 6.267-2.257 6.267-6.268S13.611 3.333 9.6 3.333 3.333 5.589 3.333 9.6s2.256 6.268 6.267 6.268"
        stroke="currentColor"
        strokeWidth={2.92}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </Svg>
  );
};
