import { Spinner } from "./Spinner";
import { IconChecked } from "./icons/IconChecked";
import { IconClose } from "./icons/IconClose";

import type { ToastProps } from "#utils/toaster.js";

const Icon = {
  success: (
    <div className="min-h-6 min-w-6 rounded-full bg-surface-quaternary-green text-green-bean-100 flex items-center justify-center">
      <IconChecked className="w-3 h-3" />
    </div>
  ),
  error: (
    <div className="min-h-6 min-w-6 rounded-full bg-red-bean-300 text-red-bean-100 flex items-center justify-center">
      <IconClose className="w-4 h-4" />
    </div>
  ),
  loading: (
    <div className="text-blue-500 min-h-6 min-w-6  flex items-center justify-center">
      <Spinner size="sm" color="current" />
    </div>
  ),
};

export const Toast: React.FC<ToastProps> = ({ title, description, type, close }) => {
  return (
    <div className="w-fit min-w-[12rem] max-w-[20rem] py-4 pl-4 pr-10 rounded-[20px] bg-surface-primary-rice border border-secondary-gray transition-all duration-500 shadow-account-card flex items-start gap-2 relative">
      {Icon[type]}
      <div className="flex flex-1 flex-col overflow-hidden min-w-0">
        <p className="text-primary-900 diatype-sm-medium">{title}</p>
        {description && (
          <p className="text-tertiary-500 diatype-xs-medium break-all">{description}</p>
        )}
      </div>
      <button
        aria-label="Close Notification"
        className="absolute top-4 right-4 transition-all duration-200"
        onClick={close}
        type="button"
      >
        <IconClose className="w-6 h-6 text-tertiary-500 hover:text-primary-900" />
      </button>
    </div>
  );
};
