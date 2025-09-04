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
  promise: <T>(
    promise: Promise<T>,
    toastMsgs?: {
      loading?: ToastMsg;
      success?: ToastMsg;
      error?: ToastMsg;
    },
    delay?: number,
  ) => Promise<void | T>;
  success: (toastMsg?: ToastMsg, options?: ToastOptions) => string;
  error: (toastMsg?: ToastMsg, options?: ToastOptions) => string;
  loading: (toastMsg?: ToastMsg, options?: ToastOptions) => string;
};
