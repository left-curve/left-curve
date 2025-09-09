import { Svg } from "./SvgBase";
import { Path } from "react-native-svg";

import type React from "react";
import type { SvgProps } from "react-native-svg";

export const IconClose: React.FC<SvgProps> = ({ ...props }) => {
  return (
    <Svg width="24" height="24" fill="none" viewBox="0 0 24 24" {...props}>
      <Path
        fill="currentColor"
        fillRule="evenodd"
        d="M6.514 6.37a1.5 1.5 0 0 1 2.116.144 91.5 91.5 0 0 0 8.856 8.856 1.5 1.5 0 0 1-1.972 2.26A94.5 94.5 0 0 1 6.37 8.487a1.5 1.5 0 0 1 .144-2.116"
        clipRule="evenodd"
      />
      <Path
        fill="currentColor"
        fillRule="evenodd"
        d="M17.486 6.37a1.5 1.5 0 0 1 .145 2.116 94.5 94.5 0 0 1-9.145 9.145 1.5 1.5 0 0 1-1.972-2.261 91.6 91.6 0 0 0 8.856-8.856 1.5 1.5 0 0 1 2.116-.144"
        clipRule="evenodd"
      />
    </Svg>
  );
};
