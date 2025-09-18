import { create } from "zustand";

import type {
  ToastDefinition,
  ToastMessage,
  ToastOptions,
  ToastStore,
} from "@left-curve/foundation";

const toastStore = create<ToastStore>((set, get) => ({
  toasts: [] as ToastDefinition[],
  add(type, message, options) {
    const { id, duration = 4000 } = options || {};
    const { title, description } = message;
    const createdAt = Date.now();

    const toast: ToastDefinition = {
      id: id || createdAt.toString(),
      title,
      description,
      type,
      createdAt,
      duration,
    };

    const { toasts } = get();

    set({ toasts: [toast, ...toasts] });
    setTimeout(() => {
      set({ toasts: get().toasts.filter((t) => t.id !== toast.id) });
    }, duration);
    return toast.id;
  },
  remove(id) {
    set({ toasts: get().toasts.filter((t) => t.id !== id) });
  },
}));

export const toast = {
  getState() {
    return toastStore.getState();
  },
  error(message: ToastMessage, options?: ToastOptions) {
    return this.getState().add("error", message, options);
  },
  success(message: ToastMessage, options?: ToastOptions) {
    return this.getState().add("success", message, options);
  },
  dismiss(id: string) {
    this.getState().remove(id);
  },
};

export { toastStore as useToastStore };
