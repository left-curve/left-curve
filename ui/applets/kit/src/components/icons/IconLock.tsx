import type React from "react";

export const IconLock: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      fill="none"
      viewBox="0 0 24 24"
      {...props}
    >
      <path
        fill="currentColor"
        fillRule="evenodd"
        d="M8.527 4.425a4.864 4.864 0 0 1 8.304 3.44v1.109a4.47 4.47 0 0 1 2.7 3.569c.087.725.15 1.475.15 2.243s-.063 1.517-.15 2.242a4.465 4.465 0 0 1-4.245 3.912c-1.06.04-2.146.06-3.32.06-1.175 0-2.261-.02-3.32-.06A4.465 4.465 0 0 1 4.4 17.028a19 19 0 0 1-.15-2.242c0-.768.063-1.518.15-2.243a4.47 4.47 0 0 1 2.702-3.57V7.864c0-1.29.513-2.527 1.425-3.44m6.375 3.44v.753a90 90 0 0 0-2.936-.047c-1.033 0-1.997.016-2.935.047v-.754a2.936 2.936 0 1 1 5.871 0m-1.972 6.14a.964.964 0 0 0-1.929 0v1.338a.964.964 0 1 0 1.929 0z"
        clipRule="evenodd"
      ></path>
    </svg>
  );
};
