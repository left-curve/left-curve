import { Svg } from "./SvgBase";
import { Path } from "react-native-svg";

import type React from "react";
import type { SvgProps } from "react-native-svg";

export const IconEmptyStar: React.FC<SvgProps> = ({ ...props }) => {
  return (
    <Svg width={20} height={20} viewBox="0 0 20 20" fill="none" {...props}>
      <Path
        d="M18.685 9.1c.6-.8.2-1.9-.7-2.2-1.4-.5-2.7-.7-4.1-.7h-.7l-.2-.7c-.4-1.5-1-2.7-1.9-3.9-.5-.8-1.6-.8-2.2 0-.9 1.2-1.5 2.4-1.9 3.9l-.2.7h-.8c-1.4 0-2.7.2-4 .7-.9.3-1.3 1.4-.7 2.2.8 1.2 1.8 2.2 3 3.1l.6.4-.2.7c-.5 1.4-.7 2.8-.7 4.2 0 1 .9 1.7 1.8 1.4 1.3-.4 2.5-1 3.6-1.9l.6-.4.6.4c1.1.9 2.3 1.5 3.6 1.9.9.3 1.8-.4 1.8-1.4q0-2.1-.6-4.2l-.2-.7.6-.4c1.2-.9 2.2-1.9 3-3.1"
        stroke="currentColor"
        strokeWidth={2}
      />
    </Svg>
  );
};
