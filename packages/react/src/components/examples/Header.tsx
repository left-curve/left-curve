"use client";

import { useEffect, useState } from "react";
import { Button, Modal } from "~/components";
import { useAccount, useChainId, useConnectors } from "~/hooks";

export const ExampleHeader: React.FC = () => {
  const [showModal, setShowModal] = useState(false);
  const { isConnected, username, connector } = useAccount();
  const connectors = useConnectors();
  const chainId = useChainId();

  useEffect(() => {
    if (isConnected) {
      setShowModal(false);
    }
  }, [isConnected]);

  return (
    <header className="flex h-16 w-full items-center justify-between px-4 md:px-6 bg-stone-100">
      <div className="flex items-center gap-2">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          className="w-6 h-6 text-primary-500"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="m8 3 4 8 5-5 5 15H2L8 3z" />
        </svg>
        <Modal showModal={showModal} onClose={() => setShowModal(false)}>
          <div className="flex flex-col px-4 py-8 text-neutral-100 rounded-3xl bg-neutral-700 min-h-[350px] min-w-[300px]">
            <p className="text-2xl text-center font-bold">Connect Wallet</p>
            <ul className="flex flex-col px-2 py-4 gap-4">
              {connectors.map((connector) => {
                const Icon = icons[connector.id as keyof typeof icons];
                return (
                  <Button
                    key={connector.name}
                    className="bg-neutral-600 hover:bg-neutral-500 py-6"
                    onClick={() =>
                      connector.connect({
                        username: "owner",
                        chainId,
                        challenge: "Please sign this message to confirm your identity.",
                      })
                    }
                  >
                    <span className="flex w-full items-center justify-between">
                      <span className="text-lg">{connector.name}</span>
                      <div className="flex justify-center items-center w-8 h-8">
                        {connector.icon ? (
                          <img src={connector.icon} alt={connector.name} />
                        ) : (
                          <Icon />
                        )}
                      </div>
                    </span>
                  </Button>
                );
              })}
            </ul>
          </div>
        </Modal>
        <span className="text-lg font-semibold">Example App</span>
      </div>
      <Button
        className="relative min-w-28 group"
        onClick={() => (isConnected ? connector?.disconnect() : setShowModal(true))}
      >
        {!isConnected ? <p>Connect</p> : null}
        {isConnected ? (
          <p className="text-center">
            <span className="block group-hover:hidden">{username}</span>
            <span className="hidden group-hover:block">Disconnect</span>
          </p>
        ) : null}
      </Button>
    </header>
  );
};

const PasskeyIcon: React.FC = ({ ...props }) => {
  return (
    <svg
      width="216"
      height="216"
      viewBox="0 0 216 216"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <g clipPath="url(#clip0_1_2)">
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M172.32 96.79C172.32 110.57 163.84 122.29 152.03 126.57L159.17 138.4L148.6 151.4L159.17 164.11L142.13 186.98L130.12 174.16V148.26V125.7C119.44 120.85 111.97 109.73 111.97 96.79C111.97 79.39 125.48 65.28 142.15 65.28C158.81 65.28 172.32 79.39 172.32 96.79ZM142.14 101.61C146.16 101.61 149.42 98.21 149.42 94.01C149.42 89.81 146.16 86.4 142.14 86.4C138.12 86.4 134.86 89.8 134.86 94.01C134.85 98.21 138.12 101.61 142.14 101.61Z"
          fill="white"
        />
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M172.41 96.88C172.41 110.5 164.16 122.11 152.58 126.55L159.16 138.39L149.43 151.39L159.16 164.1L142.13 187.15V161.25V128.48V101.61C146.15 101.61 149.41 98.2 149.41 94.01C149.41 89.81 146.15 86.4 142.13 86.4V65.28C158.86 65.28 172.41 79.43 172.41 96.88Z"
          fill="#DAD9D9"
        />
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M120.24 131.43C110.49 123.43 103.94 111.13 103.04 97.16H50.8C39.84 97.16 30.96 106.17 30.96 117.29V142.46C30.96 148.02 35.4 152.53 40.88 152.53H110.32C115.8 152.53 120.24 148.02 120.24 142.46V131.43Z"
          fill="white"
        />
        <path
          d="M73.16 91.13C70.74 90.67 68.34 90.24 66.05 89.27C57.4 85.64 52.36 78.95 50.73 69.5C49.61 63.03 50.14 56.63 52.76 50.58C56.48 41.98 63.15 37.32 71.91 35.74C77.15 34.8 82.37 35.01 87.41 36.89C95 39.71 100.09 45.15 102.44 53.13C104.82 61.18 104.47 69.23 100.88 76.85C97.16 84.81 90.67 89.08 82.46 90.75C81.78 90.89 81.09 91.02 80.41 91.16C78 91.13 75.58 91.13 73.16 91.13Z"
          fill="white"
        />
      </g>
      <defs>
        <clipPath id="clip0_1_2">
          <rect width="216" height="216" fill="white" />
        </clipPath>
      </defs>
    </svg>
  );
};

const KeplrIcon: React.FC = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="42"
      height="42"
      viewBox="0 0 42 42"
      fill="none"
      {...props}
    >
      <g clipPath="url(#clip0_425_5107)">
        <path
          d="M32.4545 0H9.54545C4.27365 0 0 4.27365 0 9.54545V32.4545C0 37.7264 4.27365 42 9.54545 42H32.4545C37.7264 42 42 37.7264 42 32.4545V9.54545C42 4.27365 37.7264 0 32.4545 0Z"
          fill="url(#paint0_linear_425_5107)"
        />
        <path
          d="M32.4545 0H9.54545C4.27365 0 0 4.27365 0 9.54545V32.4545C0 37.7264 4.27365 42 9.54545 42H32.4545C37.7264 42 42 37.7264 42 32.4545V9.54545C42 4.27365 37.7264 0 32.4545 0Z"
          fill="url(#paint1_radial_425_5107)"
        />
        <path
          d="M32.4545 0H9.54545C4.27365 0 0 4.27365 0 9.54545V32.4545C0 37.7264 4.27365 42 9.54545 42H32.4545C37.7264 42 42 37.7264 42 32.4545V9.54545C42 4.27365 37.7264 0 32.4545 0Z"
          fill="url(#paint2_radial_425_5107)"
        />
        <path
          d="M32.4545 0H9.54545C4.27365 0 0 4.27365 0 9.54545V32.4545C0 37.7264 4.27365 42 9.54545 42H32.4545C37.7264 42 42 37.7264 42 32.4545V9.54545C42 4.27365 37.7264 0 32.4545 0Z"
          fill="url(#paint3_radial_425_5107)"
        />
        <path
          d="M17.2526 32.2614V22.5192L26.7185 32.2614H31.9849V32.0079L21.0964 20.9122L31.1469 10.3857V10.2614H25.8464L17.2526 19.5635V10.2614H12.9849V32.2614H17.2526Z"
          fill="white"
        />
      </g>
      <defs>
        <linearGradient
          id="paint0_linear_425_5107"
          x1="21"
          y1="0"
          x2="21"
          y2="42"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#1FD1FF" />
          <stop offset="1" stopColor="#1BB8FF" />
        </linearGradient>
        <radialGradient
          id="paint1_radial_425_5107"
          cx="0"
          cy="0"
          r="1"
          gradientUnits="userSpaceOnUse"
          gradientTransform="translate(2.00623 40.4086) rotate(-45.1556) scale(67.3547 68.3624)"
        >
          <stop stopColor="#232DE3" />
          <stop offset="1" stopColor="#232DE3" stopOpacity="0" />
        </radialGradient>
        <radialGradient
          id="paint2_radial_425_5107"
          cx="0"
          cy="0"
          r="1"
          gradientUnits="userSpaceOnUse"
          gradientTransform="translate(39.7379 41.7602) rotate(-138.45) scale(42.1137 64.2116)"
        >
          <stop stopColor="#8B4DFF" />
          <stop offset="1" stopColor="#8B4DFF" stopOpacity="0" />
        </radialGradient>
        <radialGradient
          id="paint3_radial_425_5107"
          cx="0"
          cy="0"
          r="1"
          gradientUnits="userSpaceOnUse"
          gradientTransform="translate(20.6501 0.311498) rotate(90) scale(33.1135 80.3423)"
        >
          <stop stopColor="#24D5FF" />
          <stop offset="1" stopColor="#1BB8FF" stopOpacity="0" />
        </radialGradient>
        <clipPath id="clip0_425_5107">
          <rect width="42" height="42" fill="white" />
        </clipPath>
      </defs>
    </svg>
  );
};

const MetamaskIcon: React.FC = ({ ...props }) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="10em"
      height="10em"
      viewBox="0 0 256 240"
      {...props}
    >
      <path fill="#E17726" d="M250.066 0L140.219 81.279l20.427-47.9z" />
      <path
        fill="#E27625"
        d="m6.191.096l89.181 33.289l19.396 48.528zM205.86 172.858l48.551.924l-16.968 57.642l-59.243-16.311zm-155.721 0l27.557 42.255l-59.143 16.312l-16.865-57.643z"
      />
      <path
        fill="#E27625"
        d="m112.131 69.552l1.984 64.083l-59.371-2.701l16.888-25.478l.214-.245zm31.123-.715l40.9 36.376l.212.244l16.888 25.478l-59.358 2.7zM79.435 173.044l32.418 25.259l-37.658 18.181zm97.136-.004l5.131 43.445l-37.553-18.184z"
      />
      <path
        fill="#D5BFB2"
        d="m144.978 195.922l38.107 18.452l-35.447 16.846l.368-11.134zm-33.967.008l-2.909 23.974l.239 11.303l-35.53-16.833z"
      />
      <path
        fill="#233447"
        d="m100.007 141.999l9.958 20.928l-33.903-9.932zm55.985.002l24.058 10.994l-34.014 9.929z"
      />
      <path
        fill="#CC6228"
        d="m82.026 172.83l-5.48 45.04l-29.373-44.055zm91.95.001l34.854.984l-29.483 44.057zm28.136-44.444l-25.365 25.851l-19.557-8.937l-9.363 19.684l-6.138-33.849zm-148.237 0l60.435 2.749l-6.139 33.849l-9.365-19.681l-19.453 8.935z"
      />
      <path
        fill="#E27525"
        d="m52.166 123.082l28.698 29.121l.994 28.749zm151.697-.052l-29.746 57.973l1.12-28.8zm-90.956 1.826l1.155 7.27l2.854 18.111l-1.835 55.625l-8.675-44.685l-.003-.462zm30.171-.101l6.521 35.96l-.003.462l-8.697 44.797l-.344-11.205l-1.357-44.862z"
      />
      <path
        fill="#F5841F"
        d="m177.788 151.046l-.971 24.978l-30.274 23.587l-6.12-4.324l6.86-35.335zm-99.471 0l30.399 8.906l6.86 35.335l-6.12 4.324l-30.275-23.589z"
      />
      <path
        fill="#C0AC9D"
        d="m67.018 208.858l38.732 18.352l-.164-7.837l3.241-2.845h38.334l3.358 2.835l-.248 7.831l38.487-18.29l-18.728 15.476l-22.645 15.553h-38.869l-22.63-15.617z"
      />
      <path
        fill="#161616"
        d="m142.204 193.479l5.476 3.869l3.209 25.604l-4.644-3.921h-36.476l-4.556 4l3.104-25.681l5.478-3.871z"
      />
      <path
        fill="#763E1A"
        d="M242.814 2.25L256 41.807l-8.235 39.997l5.864 4.523l-7.935 6.054l5.964 4.606l-7.897 7.191l4.848 3.511l-12.866 15.026l-52.77-15.365l-.457-.245l-38.027-32.078zm-229.628 0l98.326 72.777l-38.028 32.078l-.457.245l-52.77 15.365l-12.866-15.026l4.844-3.508l-7.892-7.194l5.952-4.601l-8.054-6.071l6.085-4.526L0 41.809z"
      />
      <path
        fill="#F5841F"
        d="m180.392 103.99l55.913 16.279l18.165 55.986h-47.924l-33.02.416l24.014-46.808zm-104.784 0l-17.151 25.873l24.017 46.808l-33.005-.416H1.631l18.063-55.985zm87.776-70.878l-15.639 42.239l-3.319 57.06l-1.27 17.885l-.101 45.688h-30.111l-.098-45.602l-1.274-17.986l-3.32-57.045l-15.637-42.239z"
      />
    </svg>
  );
};

const icons: Record<string, React.FC> = {
  metamask: MetamaskIcon,
  keplr: KeplrIcon,
  passkey: PasskeyIcon,
};
