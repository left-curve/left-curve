// IconSun.native.tsx
import type React from "react";
import { useId } from "react";
import Svg, { G, Path, Mask, Defs, ClipPath, type SvgProps } from "react-native-svg";

export const IconSun: React.FC<SvgProps & { size?: number }> = ({
  color = "#2E2521",
  size = 24,
  width,
  height,
  ...props
}) => {
  const id = useId();
  const clipId = `clip-${id}`;
  const maskId = `mask-${id}`;

  return (
    <Svg
      width={size || width || 24}
      height={size || height || 24}
      viewBox="0 0 24 24"
      color={color}
      {...props}
    >
      <G clipPath={`url(#${clipId})`}>
        <Mask id={maskId} x={0} y={0} width={24} height={24} maskUnits="userSpaceOnUse">
          <Path fill="#fff" d="M24 0H0v24h24z" />
        </Mask>

        <G fill="currentColor" mask={`url(#${maskId})`}>
          <Path d="M15.536 8.464a5 5 0 1 1-7.071 7.072 5 5 0 0 1 7.07-7.072" />
          <Path
            fillRule="evenodd"
            clipRule="evenodd"
            d="M12 .5A1.5 1.5 0 0 1 13.5 2v2a1.5 1.5 0 0 1-3 0V2A1.5 1.5 0 0 1 12 .5M12 18.5a1.5 1.5 0 0 1 1.5 1.5v2a1.5 1.5 0 0 1-3 0v-2a1.5 1.5 0 0 1 1.5-1.5M20.13 3.869a1.5 1.5 0 0 1 0 2.121l-.71.71A1.5 1.5 0 0 1 17.3 4.58l.71-.71a1.5 1.5 0 0 1 2.12 0M6.7 17.299a1.5 1.5 0 0 1 0 2.121l-.71.71a1.5 1.5 0 0 1-2.12-2.121l.71-.71a1.5 1.5 0 0 1 2.12 0M18.5 12a1.5 1.5 0 0 1 1.5-1.5h2a1.5 1.5 0 0 1 0 3h-2a1.5 1.5 0 0 1-1.5-1.5M.5 12A1.5 1.5 0 0 1 2 10.5h2a1.5 1.5 0 0 1 0 3H2A1.5 1.5 0 0 1 .5 12M17.3 17.299a1.5 1.5 0 0 1 2.12 0l.71.71a1.5 1.5 0 0 1-2.12 2.121l-.71-.71a1.5 1.5 0 0 1 0-2.121M3.87 3.869a1.5 1.5 0 0 1 2.12 0l.71.71A1.5 1.5 0 1 1 4.58 6.7l-.71-.71a1.5 1.5 0 0 1 0-2.121"
          />
        </G>
      </G>

      <Defs>
        <ClipPath id={clipId}>
          <Path fill="#fff" d="M0 0h24v24H0z" />
        </ClipPath>
      </Defs>
    </Svg>
  );
};
