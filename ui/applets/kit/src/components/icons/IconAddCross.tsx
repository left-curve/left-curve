import type React from "react";

export const IconAddCross: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
        d="M10.74 18.5V5.5C10.74 4.80964 11.2996 4.25 11.99 4.25C12.6803 4.25 13.24 4.80964 13.24 5.5V18.5C13.24 19.1904 12.6803 19.75 11.99 19.75C11.2996 19.75 10.74 19.1904 10.74 18.5Z"
        fill="currentColor"
      />
      <path
        d="M18.49 10.751C19.1803 10.751 19.74 11.3106 19.74 12.001C19.74 12.6913 19.1803 13.251 18.49 13.251H5.48999C4.79963 13.251 4.23999 12.6913 4.23999 12.001C4.23999 11.3106 4.79963 10.751 5.48999 10.751H18.49Z"
        fill="currentColor"
      />
    </svg>
  );
};
