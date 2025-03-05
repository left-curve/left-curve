export const EthereumIcon: React.FC<React.SVGAttributes<HTMLOrSVGElement>> = ({ ...props }) => {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="25" height="24" fill="none" viewBox="0 0 25 24">
      <path
        fill="currentColor"
        d="M12.866 9.136l-6.752 3.07 6.752 3.992 6.755-3.993-6.755-3.07z"
        opacity="0.6"
        {...props}
      />
      <path fill="currentColor" d="M6.117 12.203l6.752 3.992V1L6.117 12.203z" opacity="0.45" />
      <path fill="currentColor" d="M12.869 1v15.195l6.752-3.992L12.869 1z" opacity="0.8" />
      <path fill="currentColor" d="M6.114 13.483L12.866 23v-5.526l-6.752-3.99z" opacity="0.45" />
      <path fill="currentColor" d="M12.866 17.474V23l6.757-9.517-6.757 3.99z" opacity="0.8" />
    </svg>
  );
};
