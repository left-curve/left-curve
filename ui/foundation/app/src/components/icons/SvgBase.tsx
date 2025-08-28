import { Svg as RNSvg } from "react-native-svg";

import { cssInterop } from "nativewind";

import type React from "react";
import type { PropsWithChildren } from "react";
import type { SvgProps } from "react-native-svg";

cssInterop(RNSvg, {
  className: {
    target: "style",
    nativeStyleToProp: {
      fill: true,
      color: true,
      stroke: true,
      width: true,
      height: true,
      transform: true,
    },
  },
});

export const Svg: React.FC<PropsWithChildren<SvgProps>> = ({ children, ...props }) => {
  return <RNSvg {...props}>{children}</RNSvg>;
};
