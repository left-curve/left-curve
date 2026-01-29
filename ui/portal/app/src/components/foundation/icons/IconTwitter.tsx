import { Svg } from "./SvgBase";
import { Path } from "react-native-svg";

import type { SvgProps } from "react-native-svg";
import type React from "react";

export const IconTwitter: React.FC<SvgProps> = ({ ...props }) => {
  return (
    <Svg width={24} height={24} viewBox="0 0 24 24" fill="none" {...props}>
      <Path
        d="M16.6009 5H19.0544L13.6943 11.3538L20 20H15.0627L11.1957 14.7562L6.77087 20H4.31595L10.049 13.2038L4 5H9.06262L12.5581 9.79308L16.6009 5ZM15.7399 18.4769H17.0993L8.32392 6.44308H6.86506L15.7399 18.4769Z"
        fill="currentColor"
      />
    </Svg>
  );
};
