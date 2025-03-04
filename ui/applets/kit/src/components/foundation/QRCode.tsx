import QRCodeStyling, { type Options as QROptions } from "qr-code-styling";
import { useEffect, useRef } from "react";
import { twMerge } from "../../utils";

const defaultOptions: QROptions = {
  width: 180,
  height: 180,
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
  data: string;
  options?: QROptions;
}

export const QRCode: React.FC<Props> = ({ data, options = {}, ...props }) => {
  const qrCode = new QRCodeStyling({ ...defaultOptions, ...options });
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    qrCode.append(ref.current as HTMLDivElement);
  }, []);

  useEffect(() => {
    qrCode.update({ data });
  }, [data]);

  return <div ref={ref} {...props} className={twMerge("bg-rice-25 p-2", props.className)} />;
};
