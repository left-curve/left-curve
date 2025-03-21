import { useState } from "react";
import hotToast from "react-hot-toast";

import { IconChecked, IconClose, Spinner } from "@left-curve/applets-kit";
import { wait } from "@left-curve/dango/utils";

interface ToastMsg {
  title?: string;
  description?: string;
}

interface ToastOptions {
  id?: string;
  duration?: number;
}

interface Props {
  title: string;
  type: "error" | "success" | "loading";
  close: () => void;
  description?: string;
}

const Icon = {
  success: (
    <div className="min-h-6 min-w-6 rounded-full bg-green-bean-300 text-green-bean-100 flex items-center justify-center">
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

export const Toast: React.FC<Props> = ({ title, description, type, close }) => {
  return (
    <div className="w-fit min-w-[12rem] max-w-[20rem] p-4 rounded-[20px] bg-white-100 border border-gray-100 transition-all duration-500 shadow-card-shadow flex items-center gap-2">
      {Icon[type]}
      <div className="flex flex-col">
        <p className="text-gray-900 diatype-sm-medium">{title}</p>
        {description && <p className="text-gray-500 diatype-xs-medium">{description}</p>}
      </div>
    </div>
  );
};

const success = (toastMsg?: ToastMsg, options?: ToastOptions) =>
  hotToast.custom((t) => {
    const msg = Object.assign({ title: "Operation Sucessful" }, toastMsg);
    return (
      <Toast
        close={() => hotToast.dismiss(t.id)}
        title={msg.title}
        description={msg.description}
        type="success"
      />
    );
  }, options);

const error = (toastMsg?: ToastMsg, options?: ToastOptions) =>
  hotToast.custom((t) => {
    const msg = Object.assign(
      { title: "Error", description: "Something went wrong. Please try again later." },
      toastMsg,
    );
    return (
      <Toast
        close={() => hotToast.dismiss(t.id)}
        title={msg.title}
        description={msg.description}
        type="error"
      />
    );
  }, options);

const loading = (toastMsg?: ToastMsg, options?: ToastOptions) =>
  hotToast.custom((t) => {
    const msg = Object.assign({ title: "Loading..." }, toastMsg);
    return (
      <Toast
        close={() => hotToast.dismiss(t.id)}
        title={msg.title}
        description={msg.description}
        type="loading"
      />
    );
  }, options);

const promise = async <T,>(
  promise: Promise<T>,
  toastMsgs?: { loading?: ToastMsg; success?: ToastMsg; error?: ToastMsg },
  delay?: number,
) => {
  const id = loading(toastMsgs?.loading, { duration: Number.POSITIVE_INFINITY });

  return promise
    .then(async (result) => {
      if (delay) await wait(delay);
      success(toastMsgs?.success, { id, duration: 2000 });
      return result;
    })
    .catch((e) => {
      error(toastMsgs?.error, { id, duration: 2000 });
      console.log(e);
    });
};

export const toast = {
  promise,
  success,
  error,
  loading,
};
