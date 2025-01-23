import type React from "react";

export const ProfileIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 20 20"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <g clipPath="url(#clip0_2348_48637)">
        <path d="M10 20C16.4 20 20 16.4 20 10C20 3.6 16.4 0 10 0C3.6 0 0 3.6 0 10C0 16.4 3.6 20 10 20Z" />
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M13.3657 7.52685C13.3657 9.68113 12.1543 10.8926 10 10.8926C7.84572 10.8926 6.63429 9.68113 6.63429 7.52685C6.63429 5.37256 7.84572 4.16113 10 4.16113C12.1543 4.16113 13.3657 5.37256 13.3657 7.52685ZM10 19.9997C12.6543 19.9997 14.8286 19.3811 16.4614 18.1997C16.0209 16.8325 15.158 15.6402 13.9969 14.7945C12.8359 13.9488 11.4364 13.4931 10 13.4931C8.56356 13.4931 7.16414 13.9488 6.00306 14.7945C4.84199 15.6402 3.97912 16.8325 3.53857 18.1997C5.17143 19.3783 7.34572 19.9997 10 19.9997Z"
          fill="#402B3B"
        />
      </g>
      <defs>
        <clipPath id="clip0_2348_48637">
          <rect width="20" height="20" fill="white" />
        </clipPath>
      </defs>
    </svg>
  );
};
