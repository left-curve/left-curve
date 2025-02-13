import type React from "react";

export const ProfileIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="48"
      height="48"
      fill="currentColor"
      viewBox="0 0 48 48"
      {...props}
    >
      <path
        fillRule="evenodd"
        d="M33.523 11.769c0 6.252-3.516 9.769-9.77 9.769-6.252 0-9.768-3.517-9.768-9.77C13.985 5.517 17.501 2 23.754 2s9.769 3.516 9.769 9.769m-9.77 33.353c7.705 0 18.755 0 18.755-5.224 0-3.898-3.783-7.43-7.153-9.884a19.7 19.7 0 0 0-23.202 0C8.783 32.47 5 36 5 39.898c0 5.224 11.05 5.224 18.754 5.224"
        clipRule="evenodd"
      />
    </svg>
  );
};
