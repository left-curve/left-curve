"use client";

import { useAccount } from "@leftcurve/react";
import { useRef } from "react";
import { Button, ConnectModal } from "../";
import type { ModalRef } from "./Modal";

export const ConnectButton: React.FC = () => {
  const modalRef = useRef<ModalRef>(null);
  const { username, connector, isConnected } = useAccount();

  return (
    <>
      <Button color="gray" className="relative group" onClick={() => modalRef.current?.showModal()}>
        {isConnected ? <p>Connect</p> : null}
        {!isConnected ? (
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
