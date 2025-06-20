import { wait } from "@left-curve/dango/utils";
import { Toaster, toast as hotToast } from "react-hot-toast";
import type { ToasterProps } from "react-hot-toast";

export type ToastMsg = {
  title?: string;
  description?: string;
};

export type ToastOptions = {
  id?: string;
  duration?: number;
};

export type ToastProps = {
  title: string;
  type: "error" | "success" | "loading";
  close: () => void;
  description?: string;
};

export type ToastController = {
  promise: ReturnType<typeof promise>;
  success: ReturnType<typeof success>;
  error: ReturnType<typeof error>;
  loading: ReturnType<typeof loading>;
};

const success =
  (toast: (props: ToastProps) => JSX.Element) => (toastMsg?: ToastMsg, options?: ToastOptions) =>
    hotToast.custom((t) => {
      const msg = Object.assign({ title: "Operation Sucessful" }, toastMsg);
      return toast({
        close: () => hotToast.dismiss(t.id),
        title: msg.title,
        description: msg.description,
        type: "success",
      });
    }, options);

const error =
  (toast: (props: ToastProps) => JSX.Element) => (toastMsg?: ToastMsg, options?: ToastOptions) =>
    hotToast.custom(
      (t) => {
        const msg = Object.assign(
          { title: "Error", description: "Something went wrong. Please try again later." },
          toastMsg,
        );
        return toast({
          close: () => hotToast.dismiss(t.id),
          title: msg.title,
          description: msg.description,
          type: "error",
        });
      },
      { ...options, duration: Number.POSITIVE_INFINITY },
    );

const loading =
  (toast: (props: ToastProps) => JSX.Element) => (toastMsg?: ToastMsg, options?: ToastOptions) =>
    hotToast.custom((t) => {
      const msg = Object.assign({ title: "Loading..." }, toastMsg);
      return toast({
        close: () => hotToast.dismiss(t.id),
        title: msg.title,
        description: msg.description,
        type: "loading",
      });
    }, options);

const promise =
  (toast: (props: ToastProps) => JSX.Element) =>
  async <T>(
    promise: Promise<T>,
    toastMsgs?: { loading?: ToastMsg; success?: ToastMsg; error?: ToastMsg },
    delay?: number,
  ) => {
    const id = loading(toast)(toastMsgs?.loading, { duration: Number.POSITIVE_INFINITY });

    return promise
      .then(async (result) => {
        if (delay) await wait(delay);
        success(toast)(toastMsgs?.success, { id, duration: 2000 });
        return result;
      })
      .catch((e) => {
        error(toast)(toastMsgs?.error, { id, duration: 2000 });
        console.log(e);
      });
  };

export function createToaster(
  toast: (props: ToastProps) => JSX.Element,
): [React.FC<ToasterProps>, ToastController] {
  return [
    Toaster,
    {
      promise: promise(toast),
      success: success(toast),
      error: error(toast),
      loading: loading(toast),
    },
  ];
}
