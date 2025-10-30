import { useZxing } from "react-zxing";
import { useRef } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import { useDOMRef } from "@left-curve/applets-kit";

type QRScanProps = {
  onScan: (result: string) => void;
};

export const QRScan: React.FC<QRScanProps> = ({ onScan }) => {
  const isAlreadyScanned = useRef(false);
  const { ref } = useZxing({
    onError: (error) => console.error(error),
    onDecodeError: (error) => console.error(error),
    onDecodeResult(rawValue) {
      const value = rawValue.getText();
      console.log("Scanned QR code:", value);
      const socketId = value.split("socketId=")[1];
      if (!socketId) return;
      if (isAlreadyScanned.current) return;
      isAlreadyScanned.current = true;
      onScan(socketId);
    },
  });

  const videoRef = useDOMRef<HTMLVideoElement>(ref);
  return (
    <>
      {/** biome-ignore lint/a11y/useMediaCaption: there is not need to add captions for QR scan */}
      <video ref={videoRef} className="flex h-fit px-2 p-4" />
      <div className="flex justify-center items-center">
        <p className="diatype-m-medium text-ink-tertiary-500 p-4 text-center">
          {m["signin.qrInstructions"]({ domain: window.location.hostname })}
        </p>
      </div>
      <div className="py-20 flex items-center justify-center">
        <p className="text-ink-tertiary-500 diatype-m-medium" />
      </div>
    </>
  );
};
