import { m } from "~/paraglide/messages";

import { Button } from "@left-curve/applets-kit";
import { Scanner } from "@yudiel/react-qr-scanner";
import { Sheet } from "react-modal-sheet";

import type React from "react";

interface Props {
  onScan: (result: string) => void;
  isVisisble: boolean;
  onClose: () => void;
}

export const QRScan: React.FC<Props> = ({ onScan, onClose, isVisisble }) => {
  return (
    <Sheet isOpen={isVisisble} onClose={onClose}>
      <Sheet.Container className="!bg-white-100 !rounded-t-2xl !shadow-none">
        <Sheet.Header className="flex items-center justify-between w-full">
          <Button variant="link" onClick={onClose}>
            {m["common.cancel"]()}
          </Button>
          <p className="mt-1 text-gray-500 font-semibold">Scan QR Code</p>
          <div className="w-[66px]" />
        </Sheet.Header>
        <Sheet.Content>
          <div className="flex justify-center items-center py-12">
            <p className="diatype-m-medium text-gray-400 p-4 text-center">
              {m["signin.qrInstructions"]({ domain: window.location.hostname })}
            </p>
          </div>
          <Scanner
            onScan={([{ rawValue }]) => onScan(rawValue)}
            allowMultiple={false}
            components={{ audio: false }}
            formats={["qr_code"]}
            classNames={{ container: "qr-container", video: "bg-white-100" }}
          />
          <div className="py-20 flex items-center justify-center">
            <p className="text-gray-400 diatype-m-medium" />
          </div>
        </Sheet.Content>
      </Sheet.Container>
      <Sheet.Backdrop onTap={onClose} />
    </Sheet>
  );
};
