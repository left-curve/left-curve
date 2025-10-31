import QRCodeStyling, { type Options as QROptions } from "qr-code-styling";
import { useEffect, useRef } from "react";
import { twMerge } from "@left-curve/foundation";
import { Spinner } from "./Spinner";

const defaultOptions: QROptions = {
  shape: "square",
  backgroundOptions: { color: "#FFF9F0" },
  cornersSquareOptions: {
    type: "square",
    color: "#000",
    gradient: {
      type: "linear",
      rotation: 90,
      colorStops: [
        { offset: 0, color: "#000" },
        { offset: 1, color: "#000" },
      ],
    },
  },
  cornersDotOptions: {
    type: "square",
    color: "#000",
  },
  imageOptions: {
    hideBackgroundDots: true,
    margin: 5,
  },
};

interface Props extends React.HTMLAttributes<HTMLDivElement> {
  data?: string;
  options?: QROptions;
  isLoading?: boolean;
}

export const QRCode: React.FC<Props> = ({ data, isLoading, options = {}, ...props }) => {
  const qrCode = new QRCodeStyling({ ...defaultOptions, ...options });
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (ref.current) {
      qrCode.append(ref.current as HTMLDivElement);
    }
  }, [isLoading]);

  useEffect(() => {
    if (!data) return;
    qrCode.update({ data });
  }, [data, isLoading]);

  return isLoading || !data ? (
    <Spinner color="blue" size="xl" />
  ) : (
    <div
      ref={ref}
      {...props}
      className={twMerge("bg-white p-2 border-2 border-outline-primary-rice rounded-md")}
    />
  );
};
