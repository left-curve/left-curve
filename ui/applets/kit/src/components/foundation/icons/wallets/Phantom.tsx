export const PhantomIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="25" height="24" fill="none" viewBox="0 0 25 24">
      <g clipPath="url(#clip0_358_42391)">
        <mask
          id="mask0_358_42391"
          style={{ maskType: "luminance" }}
          width="25"
          height="24"
          x="0"
          y="0"
          maskUnits="userSpaceOnUse"
          fill="currentColor"
          {...props}
        >
          <path fill="#fff" d="M24.5 0H.5v24h24V0z" />
        </mask>
        <g mask="url(#mask0_358_42391)">
          <path
            fill="currentColor"
            fillRule="evenodd"
            d="M10.833 15.552c-1.009 1.545-2.698 3.5-4.947 3.5-1.062 0-2.084-.437-2.084-2.338 0-4.84 6.608-12.333 12.74-12.333 3.488 0 4.878 2.42 4.878 5.169 0 3.527-2.29 7.56-4.565 7.56-.722 0-1.076-.396-1.076-1.025 0-.164.027-.342.082-.533-.777 1.326-2.276 2.557-3.68 2.557-1.021 0-1.54-.643-1.54-1.545 0-.328.069-.67.192-1.012zm8.284-6.098c0 .801-.472 1.201-1 1.201-.537 0-1.002-.4-1.002-1.2 0-.802.465-1.202 1.001-1.202.529 0 1.001.4 1.001 1.201zm-3.003 0c0 .801-.472 1.201-1 1.201-.537 0-1.002-.4-1.002-1.2 0-.802.465-1.202 1.001-1.202.529 0 1.001.4 1.001 1.201z"
            clipRule="evenodd"
          />
        </g>
      </g>
      <defs>
        <clipPath id="clip0_358_42391">
          <path fill="#fff" d="M0 0H24V24H0z" transform="translate(.5)" />
        </clipPath>
      </defs>
    </svg>
  );
};
