"use client";

import { AnimatePresence, motion } from "framer-motion";

import { type PropsWithChildren, forwardRef, useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom";

export type ModalRef = {
  closeModal: () => void;
  showModal: () => void;
};

export type ModalProps = {
  onClose?: () => void;
};

export const Modal = forwardRef<ModalRef, PropsWithChildren<ModalProps>>(
  ({ children, onClose }, ref) => {
    const modalRef = useRef<HTMLDivElement>(null);
    const [isModalOpen, setIsModalOpen] = useState(false);

    const closeModal = () => {
      setIsModalOpen(false);
      onClose?.();
    };
    const showModal = () => setIsModalOpen(true);

    useEffect(() => {
      if (!ref) return;
      if (typeof ref === "function") {
        ref({ closeModal, showModal });
      } else {
        ref.current = { closeModal, showModal };
      }
    }, [ref]);

    if (!isModalOpen) return null;

    return ReactDOM.createPortal(
      <AnimatePresence>
        <motion.div
          ref={modalRef}
          className="flex fixed w-screen h-screen backdrop-blur-[10px] inset-0 z-[60] overflow-x-auto justify-center items-center p-4"
          onClick={(e) => modalRef.current === e.target && closeModal()}
        >
          {children}
        </motion.div>
      </AnimatePresence>,
      document.querySelector("body")!,
    );
  },
);

Modal.displayName = "Modal";
