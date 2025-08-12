import type React from "react";

export const IconMirror: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg viewBox="0 0 144 185" xmlns="http://www.w3.org/2000/svg" {...props}>
      <path
        d="M0 71.6129C0 32.0622 32.0622 0 71.6129 0C111.164 0 143.226 32.0622 143.226 71.6129V174.118C143.226 180.128 138.354 185 132.343 185H10.8824C4.87222 185 0 180.128 0 174.118V71.6129Z"
        fill="url(#:r0:)"
      ></path>
      <path
        clipRule="evenodd"
        d="M134.717 176.111V71.8216C134.717 36.8684 106.465 8.53326 71.6129 8.53326C36.7613 8.53326 8.50846 36.8684 8.50846 71.8216V176.111C8.50846 176.308 8.66719 176.467 8.86298 176.467H134.363C134.559 176.467 134.717 176.308 134.717 176.111ZM71.6129 0C32.0622 0 0 32.1556 0 71.8216V176.111C0 181.02 3.96809 185 8.86298 185H134.363C139.258 185 143.226 181.02 143.226 176.111V71.8216C143.226 32.1556 111.164 0 71.6129 0Z"
        fill="currentColor"
        fillRule="evenodd"
      ></path>
      <defs>
        <linearGradient
          gradientUnits="userSpaceOnUse"
          id=":r0:"
          x1="18.435"
          x2="143.747"
          y1="10.6666"
          y2="209.447"
        >
          <stop offset="0.265625" stopColor="rgb(245 117 137)"></stop>
          <stop offset="0.734375" stopColor="currentColor"></stop>
        </linearGradient>
      </defs>
    </svg>
  );
};
