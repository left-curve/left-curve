import type React from "react";

export const IconUserCircle: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <rect x="0.5" y="0.5" width="23" height="23" rx="11.5" stroke="currentColor" />
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M16.1662 8.89884C16.1662 11.6344 14.6279 13.1727 11.8923 13.1727C9.15678 13.1727 7.61849 11.6344 7.61849 8.89884C7.61849 6.16329 9.15678 4.625 11.8923 4.625C14.6279 4.625 16.1662 6.16329 16.1662 8.89884ZM11.8923 23.4908C15.2628 23.4908 20.0972 23.4908 20.0972 21.2052C20.0972 19.5 18.4421 17.9551 16.9677 16.8812C15.4934 15.8072 13.7164 15.2287 11.8923 15.2287C10.0683 15.2287 8.2913 15.8072 6.81695 16.8812C5.3426 17.9551 3.6875 19.5 3.6875 21.2052C3.6875 23.4908 8.52188 23.4908 11.8923 23.4908Z"
        fill="currentColor"
      />
    </svg>
  );
};
