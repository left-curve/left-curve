import { Scanner } from "@yudiel/react-qr-scanner";
import { useRef } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

type QRScanProps = {
  onScan: (result: string) => void;
};

export const QRScan: React.FC<QRScanProps> = ({ onScan }) => {
  const isAlreadyScanned = useRef(false);
  return (
    <>
      <div className="flex justify-center items-center py-12">
        <p className="diatype-m-medium text-ink-tertiary-500 p-4 text-center">
          {m["signin.qrInstructions"]({ domain: window.location.hostname })}
        </p>
      </div>
      <Scanner
        onScan={([{ rawValue }]) => {
          const socketId = rawValue.split("socketId=")[1];
          if (!socketId) return;
          if (isAlreadyScanned.current) return;
          isAlreadyScanned.current = true;
          onScan(socketId);
        }}
        components={{ audio: false }}
        formats={["qr_code"]}
        classNames={{ container: "qr-container", video: "bg-surface-primary-rice" }}
      />
      <div className="py-20 flex items-center justify-center">
        <p className="text-ink-tertiary-500 diatype-m-medium" />
      </div>
    </>
  );
};
