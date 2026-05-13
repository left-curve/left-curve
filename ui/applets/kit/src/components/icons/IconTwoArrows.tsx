import type React from "react";

export const IconTwoArrows: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="21"
      height="22"
      fill="none"
      viewBox="0 0 21 22"
      {...props}
    >
      <g filter="url(#filter0_dii_13016_11409)">
        <rect
          width="20"
          height="20"
          x="0.5"
          fill="currentColor"
          rx="10"
          shapeRendering="crispEdges"
        ></rect>
        <rect
          width="19"
          height="19"
          x="1"
          y="0.5"
          stroke="#837D7B"
          rx="9.5"
          shapeRendering="crispEdges"
        ></rect>
        <g clipPath="url(#clip0_13016_11409)">
          <path
            fill="#837D7B"
            d="M6.995 13V6.636q-.258.298-.4.742a.625.625 0 1 1-1.19-.38c.327-1.025 1.007-1.74 2.01-2.088l.05-.015a.63.63 0 0 1 .36.015c1.003.348 1.683 1.063 2.01 2.087a.625.625 0 0 1-1.19.38 2.1 2.1 0 0 0-.4-.741V13c0 .508.389.875.815.875h.72a.625.625 0 1 1 0 1.25h-.72c-1.164 0-2.065-.976-2.065-2.125m5.76-6c0-.508-.389-.875-.815-.875h-.72a.625.625 0 1 1 0-1.25h.72c1.164 0 2.065.976 2.065 2.125v6.364a2.1 2.1 0 0 0 .4-.742.625.625 0 1 1 1.19.38c-.327 1.025-1.007 1.74-2.01 2.088a.63.63 0 0 1-.41 0c-1.003-.348-1.683-1.063-2.01-2.087a.625.625 0 1 1 1.19-.38q.142.441.4.74z"
          ></path>
        </g>
      </g>
      <defs>
        <clipPath id="clip0_13016_11409">
          <path fill="#fff" d="M4.5 4h12v12h-12z"></path>
        </clipPath>
        <filter
          id="filter0_dii_13016_11409"
          width="21"
          height="23"
          x="0"
          y="-1"
          colorInterpolationFilters="sRGB"
          filterUnits="userSpaceOnUse"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix"></feFlood>
          <feColorMatrix
            in="SourceAlpha"
            result="hardAlpha"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
          ></feColorMatrix>
          <feMorphology
            in="SourceAlpha"
            radius="0.5"
            result="effect1_dropShadow_13016_11409"
          ></feMorphology>
          <feOffset dy="1"></feOffset>
          <feGaussianBlur stdDeviation="0.5"></feGaussianBlur>
          <feComposite in2="hardAlpha" operator="out"></feComposite>
          <feColorMatrix values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.04 0"></feColorMatrix>
          <feBlend in2="BackgroundImageFix" result="effect1_dropShadow_13016_11409"></feBlend>
          <feBlend in="SourceGraphic" in2="effect1_dropShadow_13016_11409" result="shape"></feBlend>
          <feColorMatrix
            in="SourceAlpha"
            result="hardAlpha"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
          ></feColorMatrix>
          <feMorphology
            in="SourceAlpha"
            operator="dilate"
            radius="2"
            result="effect2_innerShadow_13016_11409"
          ></feMorphology>
          <feOffset dy="-1"></feOffset>
          <feGaussianBlur stdDeviation="1.5"></feGaussianBlur>
          <feComposite in2="hardAlpha" k2="-1" k3="1" operator="arithmetic"></feComposite>
          <feColorMatrix values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.07 0"></feColorMatrix>
          <feBlend in2="shape" result="effect2_innerShadow_13016_11409"></feBlend>
          <feColorMatrix
            in="SourceAlpha"
            result="hardAlpha"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
          ></feColorMatrix>
          <feMorphology
            in="SourceAlpha"
            operator="dilate"
            radius="1"
            result="effect3_innerShadow_13016_11409"
          ></feMorphology>
          <feOffset dy="2"></feOffset>
          <feGaussianBlur stdDeviation="1.5"></feGaussianBlur>
          <feComposite in2="hardAlpha" k2="-1" k3="1" operator="arithmetic"></feComposite>
          <feColorMatrix values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0.07 0"></feColorMatrix>
          <feBlend
            in2="effect2_innerShadow_13016_11409"
            result="effect3_innerShadow_13016_11409"
          ></feBlend>
        </filter>
      </defs>
    </svg>
  );
};
