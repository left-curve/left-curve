"use client";

import { useRef } from "react";
import { useAccount } from "~/hooks";
import { Button } from "../atoms/Button";
import { ConnectModal } from "../organisms/ConnectModal";
import type { ModalRef } from "./Modal";

export const ConnectButton: React.FC = () => {
  const modalRef = useRef<ModalRef>(null);
  const { username, connector, isConnected } = useAccount();

  return (
    <>
      <Button
        className="relative min-w-28 group"
        onClick={() => (isConnected ? connector?.disconnect() : modalRef.current?.showModal())}
      >
        {!isConnected ? <p>Connect</p> : null}
        {isConnected ? (
          <p className="text-center">
            <span className="block group-hover:hidden">{username}</span>
            <span className="hidden group-hover:block">Disconnect</span>
          </p>
        ) : null}
      </Button>
      <ConnectModal ref={modalRef} />
    </>
  );
};
