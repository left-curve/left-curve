import { useRef } from "react";
import { useQRCodeReader } from "../hooks/useQRScanner";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import { Spinner } from "./Spinner";
import { twMerge } from "@left-curve/foundation";

type QRCodeReaderProps = {
  onScan: (value: string) => void;
};

export const QRCodeReader: React.FC<QRCodeReaderProps> = ({ onScan }) => {
  const isAlreadyScanned = useRef(false);

  const { ref: videoRef, hasInitialized } = useQRCodeReader({
    onDecodeResult: (rawValue) => {
      const value = rawValue.getText();
      const socketId = value.split("socketId=")[1];
      if (!socketId) return;
      if (isAlreadyScanned.current) return;
      isAlreadyScanned.current = true;
      onScan(socketId);
    },
  });

  return (
    <>
      <div className={twMerge("flex flex-col h-full w-full", { hidden: !hasInitialized })}>
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
      </div>
      <div
        className={twMerge("flex flex-col min-h-[80vh] w-full justify-center items-center", {
          hidden: hasInitialized,
        })}
      >
        <Spinner size="lg" color="blue" fullContainer />
      </div>
    </>
  );
};
