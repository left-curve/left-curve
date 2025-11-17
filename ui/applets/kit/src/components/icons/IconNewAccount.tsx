import type React from "react";

export const IconNewAccount: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="22"
      height="22"
      fill="none"
      viewBox="0 0 22 22"
      {...props}
    >
      <g clipPath="url(#clip0_9524_42416)" clipRule="evenodd">
        <path
          fill="currentColor"
          fillRule="evenodd"
          d="M12.644 6.19c0 2.626-1.476 4.102-4.102 4.102S4.44 8.816 4.44 6.19s1.476-4.102 4.102-4.102 4.102 1.476 4.102 4.102M8.542 20.195c3.235 0 7.875 0 7.875-2.193 0-1.637-1.589-3.12-3.004-4.15a8.27 8.27 0 0 0-9.742 0c-1.415 1.03-3.004 2.513-3.004 4.15 0 2.193 4.64 2.193 7.875 2.193M13.792 8.709c0-.362.294-.656.656-.656h5.688a.656.656 0 1 1 0 1.312h-5.688a.656.656 0 0 1-.656-.656"
        />
        <path
          fill="currentColor"
          fillRule="evenodd"
          d="M17.292 12.209a.656.656 0 0 1-.656-.656V5.865a.656.656 0 0 1 1.312 0v5.688a.656.656 0 0 1-.656.656"
        />
        <path
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="0.583"
          d="M13.792 8.709c0-.362.294-.656.656-.656h5.688a.656.656 0 1 1 0 1.312h-5.688a.656.656 0 0 1-.656-.656"
        />
        <path
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="0.583"
          d="M17.292 12.209a.656.656 0 0 1-.656-.656V5.865a.656.656 0 0 1 1.312 0v5.688a.656.656 0 0 1-.656.656"
        />
      </g>
      <defs>
        <clipPath id="clip0_9524_42416">
          <path fill="#fff" d="M.667.834h21v21h-21z" />
        </clipPath>
      </defs>
    </svg>
  );
};
