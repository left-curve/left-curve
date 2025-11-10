import { AnimatePresence, motion } from "framer-motion";
import { IconClose } from "./icons/IconClose";

import { useToastStore, toast as toaster } from "../providers/toast";
import { createPortal } from "react-dom";

import type { ToastDefinition } from "@left-curve/foundation";
import type { Prettify } from "@left-curve/dango/types";
import { useMediaQuery } from "../hooks/useMediaQuery";
import { IconToastSuccess } from "./icons/IconToastSuccess";
import { IconToastWarning } from "./icons/IconToastWarning";
import { IconToastError } from "./icons/IconToastError";
import { IconToastInfo } from "./icons/IconToastInfo";

const Icon = {
  success: <IconToastSuccess className="w-6 h-6 text-utility-success-500" />,
  error: <IconToastError className="w-6 h-6 text-utility-error-500" />,
  warning: <IconToastWarning className="w-6 h-6 text-utility-warning-500" />,
  info: <IconToastInfo className="w-6 h-6 text-primitives-blue-light-500" />,
  neutral: <IconToastInfo className="w-6 h-6 text-fg-secondary-500" />,
};

export type ToastProps = Prettify<{
  toast: ToastDefinition;
}>;

export const Toast: React.FC<ToastProps> = ({ toast }) => {
  const { id, title: Title, description: Description, type } = toast;
  const { isLg } = useMediaQuery();

  const y = isLg ? 20 : -10;

  return (
    <motion.div
      initial={{ opacity: 0, y }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y }}
      className="relative w-full h-full"
    >
      <div className="absolute pointer-events-auto w-full lg:w-auto top-0 lg:top-auto lg:bottom-4 lg:right-4 bg-surface-primary-rice rounded-b-md lg:rounded-md border lg:min-w-[18rem] lg:max-w-[26rem] border-outline-secondary-gray shadow-account-card">
        <div className="w-fit py-4 pl-4 pr-10 transition-all duration-500 flex items-start gap-2">
          {Icon[type]}
          <div className="flex flex-1 flex-col overflow-hidden min-w-0">
            {typeof Title === "string" ? (
              <p className="text-ink-primary-900 diatype-sm-medium">{Title}</p>
            ) : typeof Title === "function" ? (
              <Title {...toast} />
            ) : null}

            {typeof Description === "string" ? (
              <p className="text-ink-tertiary-500 diatype-xs-medium break-all">{Description}</p>
            ) : typeof Description === "function" ? (
              <Description {...toast} />
            ) : null}
          </div>
          <button
            aria-label="Close Notification"
            className="absolute top-4 right-4 transition-all duration-200"
            onClick={() => toaster.dismiss(id)}
            type="button"
          >
            <IconClose className="w-6 h-6 text-ink-tertiary-500 hover:text-ink-primary-900" />
          </button>
        </div>
      </div>
    </motion.div>
  );
};

export const Toaster: React.FC = () => {
  const { toasts } = useToastStore();

  return createPortal(
    <div className="fixed inset-0 z-[999999] pointer-events-none">
      <AnimatePresence mode="wait">
        {toasts.map((toast) => (
          <Toast key={toast.id} toast={toast} />
        ))}
      </AnimatePresence>
    </div>,
    document.body,
  );
};
