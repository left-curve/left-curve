"use client";

import type React from "react";
import { type PropsWithChildren, useRef } from "react";
import ReactDOM from "react-dom";
import { useClickAway } from "react-use";

export type ModalProps = {
  onClose?: () => void;
  showModal: boolean;
};

export const Modal: React.FC<PropsWithChildren<ModalProps>> = ({
  onClose,
  showModal,
  children,
}) => {
  const dialogRef = useRef<HTMLDialogElement>(null);

  const closeModal = () => {
    dialogRef?.current?.close();
    onClose?.();
  };

  useClickAway(dialogRef, closeModal);

  if (!showModal) return null;

  return ReactDOM.createPortal(
    <dialog
      ref={dialogRef}
      open
      aria-modal="true"
      aria-labelledby="dialog-title"
      className="absolute flex z-[99999] top-0 left-0 right-0 bottom-0 bg-transparent"
    >
      <div className="relative">{children}</div>
    </dialog>,
    document.getElementById("modal-root")!,
  );
};

export const ModalRoot: React.FC = () => {
  return <div id="modal-root" />;
};
