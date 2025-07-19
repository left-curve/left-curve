import { Scanner } from "@yudiel/react-qr-scanner";
import type React from "react";

type QRScanProps = {
  onScan: (result: string) => void;
};

export const QRScan: React.FC<QRScanProps> = ({ onScan }) => {
  return (
    <>
      <div className="flex justify-center items-center py-12">
        <p className="diatype-m-medium text-tertiary-500 p-4 text-center" />
      </div>
      <Scanner
        onScan={([{ rawValue }]) => onScan(rawValue)}
        allowMultiple={false}
        components={{ audio: false }}
        formats={["qr_code"]}
        classNames={{ container: "qr-container", video: "bg-surface-primary-rice" }}
      />
      <div className="py-20 flex items-center justify-center">
        <p className="text-tertiary-500 diatype-m-medium" />
      </div>
    </>
  );
};
