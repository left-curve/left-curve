export const BackpackIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="25" height="24" fill="none" viewBox="0 0 25 24">
      <mask
        id="mask0_358_42400"
        style={{ maskType: "luminance" }}
        width="15"
        height="20"
        x="5"
        y="2"
        maskUnits="userSpaceOnUse"
        fill="currentColor"
        {...props}
      >
        <path fill="#fff" d="M19.625 2H5.875v20h13.75V2z" />
      </mask>
      <g mask="url(#mask0_358_42400)">
        <path
          fill="currentColor"
          fillRule="evenodd"
          d="M14.053 3.573c.726 0 1.408.097 2.04.278C15.475 2.41 14.19 2 12.763 2c-1.431 0-2.717.412-3.333 1.86a7.076 7.076 0 012.03-.287h2.592zm-2.76 1.446c-3.451 0-5.418 2.715-5.418 6.065v3.441c0 .335.28.6.625.6H19a.61.61 0 00.625-.6v-3.441c0-3.35-2.287-6.065-5.739-6.065h-2.592zm1.452 6.095a2.188 2.188 0 100-4.375 2.188 2.188 0 000 4.375zm-6.87 6.034c0-.335.28-.607.625-.607H19c.345 0 .625.272.625.607v3.639c0 .67-.56 1.213-1.25 1.213H7.125c-.69 0-1.25-.543-1.25-1.213v-3.64z"
          clipRule="evenodd"
        />
      </g>
    </svg>
  );
};
