import type React from "react";
import type { SvgProps } from "react-native-svg";
import { Path } from "react-native-svg";
import { Svg } from "./SvgBase";

export const IconReceived: React.FC<SvgProps> = ({ ...props }) => {
  return (
    <Svg width={13} height={13} viewBox="0 0 13 13" fill="none" {...props}>
      <Path
        fill="currentColor"
        fillRule="evenodd"
        clipRule="evenodd"
        d="M1.204 10.84a1.5 1.5 0 0 0 .568.567c1.154.682 2.566.99 3.832.99 1.213 0 2.676-.288 3.624-1.235A1.5 1.5 0 0 0 7.106 9.04c-.113.114-.594.357-1.502.357q-.13 0-.261-.007l6.713-6.714A1.5 1.5 0 1 0 9.935.556L3.22 7.267a5 5 0 0 1-.006-.26c0-.909.243-1.39.356-1.503a1.5 1.5 0 0 0-2.122-2.121C.502 4.33.215 5.794.215 7.007c0 1.266.307 2.679.989 3.832"
      />
    </Svg>
  );
};
