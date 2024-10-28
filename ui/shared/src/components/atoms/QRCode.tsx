import QRCodeStyling, { type Options as QROptions } from "qr-code-styling";
import { useEffect, useRef } from "react";
import { twMerge } from "../../utils";

const defaultOptions: QROptions = {
  width: 250,
  height: 250,
  dotsOptions: {
    type: "dots",
    color: "#000",
    gradient: {
      type: "radial",
      colorStops: [
        { offset: 0, color: "#000" },
        { offset: 1, color: "#000" },
      ],
    },
  },
  cornersSquareOptions: {
    type: "extra-rounded",
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
    type: "dot",
    color: "#000",
  },
  imageOptions: {
    hideBackgroundDots: true,
    margin: 5,
  },
};

interface Props extends React.HTMLAttributes<HTMLDivElement> {
  data: string;
  options?: QROptions;
}

export const QRCode: React.FC<Props> = ({ data, options = {}, ...props }) => {
  const qrCode = new QRCodeStyling({ ...defaultOptions, ...options });
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    qrCode.update({ data });
    qrCode.append(ref.current as HTMLDivElement);
  }, [qrCode, data]);

  return (
    <div ref={ref} {...props} className={twMerge("rounded-3xl bg-white p-4", props.className)} />
  );
};
