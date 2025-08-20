import React from "react";
import Svg, { Path, SvgProps } from "react-native-svg";

export const IconAddCross: React.FC<SvgProps & { size?: number }> = ({
  color = "#2E2521",
  size = 24,
  width,
  height,
  ...props
}) => {
  return (
    <Svg
      width={size || width || 24}
      height={size || height || 24}
      viewBox="0 0 48 48"
      fill="none"
      {...props}
    >
      <Path
        d="M24 12v24M36 24H12"
        stroke={color}
        strokeWidth={3.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </Svg>
  );
};
