import type React from "react";

export const IconTrust: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        d="M2.40002 4.31424L11.8665 1.20117V22.7524C5.10466 19.8787 2.40002 14.3712 2.40002 11.2586V4.31424Z"
        fill="#0500FF"
      />
      <path
        d="M21.5344 4.31424L11.8665 1.20117V22.7524C18.7721 19.8787 21.5344 14.3712 21.5344 11.2586V4.31424Z"
        fill="url(#paint0_linear_10147_320953)"
      />
      <defs>
        <linearGradient
          id="paint0_linear_10147_320953"
          x1="18.9866"
          y1="-0.301112"
          x2="11.8065"
          y2="22.5029"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0.02" stopColor="#0000FF" />
          <stop offset="0.08" stopColor="#0094FF" />
          <stop offset="0.16" stopColor="#48FF91" />
          <stop offset="0.42" stopColor="#0094FF" />
          <stop offset="0.68" stopColor="#0038FF" />
          <stop offset="0.9" stopColor="#0500FF" />
        </linearGradient>
      </defs>
    </svg>
  );
};
