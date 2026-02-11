import QRCodeStyling, { type Options as QROptions } from "qr-code-styling";
import { useEffect, useMemo, useRef } from "react";
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
  size?: number;
}

export const QRCode: React.FC<Props> = ({
  data,
  isLoading,
  options,
  size,
  className,
  ...props
}) => {
  const qrCodeRef = useRef<QRCodeStyling | null>(null);
  const ref = useRef<HTMLDivElement | null>(null);
  const mergedOptions = useMemo(
    () => ({
      ...defaultOptions,
      ...(options ?? {}),
      ...(size ? { width: size, height: size } : {}),
    }),
    [options, size],
  );

  useEffect(() => {
    if (!qrCodeRef.current) {
      qrCodeRef.current = new QRCodeStyling(mergedOptions);
      return;
    }
    qrCodeRef.current.update(mergedOptions);
  }, [mergedOptions]);

  useEffect(() => {
    if (isLoading || !ref.current || !qrCodeRef.current) return;
    qrCodeRef.current.append(ref.current);
  }, [isLoading]);

  useEffect(() => {
    if (!data) return;
    qrCodeRef.current?.update({ data });
  }, [data]);

  return isLoading || !data ? (
    <Spinner color="blue" size="xl" />
  ) : (
    <div ref={ref} {...props} className={twMerge("bg-surface-primary-rice p-2", className)} />
  );
};
