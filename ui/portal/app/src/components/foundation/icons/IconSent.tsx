import { Svg } from "./SvgBase";
import { Path } from "react-native-svg";

import type React from "react";
import type { SvgProps } from "react-native-svg";

export const IconSent: React.FC<SvgProps> = ({ ...props }) => {
  return (
    <Svg width={13} height={13} viewBox="0 0 13 13" fill="none" {...props}>
      <Path
        fill="currentColor"
        fillRule="evenodd"
        clipRule="evenodd"
        d="M11.3 1.877a1.5 1.5 0 0 0-.567-.567C9.579.628 8.167.32 6.901.32c-1.213 0-2.676.288-3.624 1.235A1.5 1.5 0 0 0 5.4 3.676c.113-.113.594-.356 1.502-.356q.13 0 .261.007L.45 10.04a1.5 1.5 0 1 0 2.121 2.122l6.713-6.714q.007.131.007.261c0 .909-.243 1.39-.356 1.503a1.5 1.5 0 0 0 2.121 2.121c.948-.947 1.235-2.41 1.235-3.624 0-1.265-.307-2.678-.99-3.832"
      />
    </Svg>
  );
};
