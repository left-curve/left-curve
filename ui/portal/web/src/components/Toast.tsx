import { useState } from "react";
import hotToast from "react-hot-toast";

import { CrossIcon } from "@left-curve/portal-shared";
import { wait } from "@left-curve/utils";

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

export const Toast: React.FC<Props> = ({ title, description, type, close }) => {
  return (
    <div className="w-full max-w-[15.75rem] p-4 rounded-[20px] text-center text-white typography-body-m font-semibold border border-borders-blue-600 backdrop-blur-sm transition-all duration-500 bg-surface-blue-600 shadow-xl">
      <p>{title}</p>
    </div>
  );
};

export const useToast = () => {
  const [isLoading, setIsLoading] = useState<boolean>(false);

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
      })
      .finally(() => {
        setIsLoading(false);
      });
  };

  return {
    isLoading,
    toast: {
      promise,
      success,
      error,
      loading,
    },
  };
};
