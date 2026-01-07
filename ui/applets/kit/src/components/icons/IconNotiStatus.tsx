import type React from "react";

export const IconNotiStatus: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <g clip-path="url(#clip0_12054_245570)">
        <circle cx="8" cy="8" r="8" fill="#FFFCF6" />
        <circle cx="7.99561" cy="7.99708" r="6.15577" fill="currentColor" />
        <path
          fill-rule="evenodd"
          clip-rule="evenodd"
          d="M12.0382 5.52525C12.3987 5.88573 12.3987 6.47019 12.0382 6.83067L7.77788 11.091C7.60477 11.2641 7.36998 11.3614 7.12516 11.3614C6.88035 11.3614 6.64556 11.2641 6.47245 11.091L4.57896 9.19754C4.21847 8.83705 4.21847 8.25259 4.57896 7.89211C4.93944 7.53163 5.5239 7.53163 5.88439 7.89211L7.12516 9.13289L10.7328 5.52525C11.0933 5.16476 11.6777 5.16476 12.0382 5.52525Z"
          fill="#FFFCF6"
        />
      </g>
      <defs>
        <clipPath id="clip0_12054_245570">
          <rect width="16" height="16" fill="white" />
        </clipPath>
      </defs>
    </svg>
  );
};
