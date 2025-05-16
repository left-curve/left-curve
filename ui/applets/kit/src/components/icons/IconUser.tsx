import type React from "react";

export const IconUser: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
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
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M16.7613 5.88439C16.7613 9.01073 15.0033 10.7688 11.877 10.7688C8.75061 10.7688 6.99256 9.01073 6.99256 5.88439C6.99256 2.75805 8.75061 1 11.877 1C15.0033 1 16.7613 2.75805 16.7613 5.88439ZM11.877 22.5609C15.7289 22.5609 21.2539 22.5609 21.2539 19.9488C21.2539 18 19.3624 16.2344 17.6774 15.0071C15.9924 13.7797 13.9615 13.1185 11.877 13.1185C9.79236 13.1185 7.76149 13.7797 6.07652 15.0071C4.39154 16.2344 2.5 18 2.5 19.9488C2.5 22.5609 8.025 22.5609 11.877 22.5609Z"
        fill="currentColor"
      />
    </svg>
  );
};
