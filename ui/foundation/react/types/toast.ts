import type { ValueOrFunction } from "@left-curve/dango/types";
import type { Renderable } from "./react";

export type ToastDefinition = {
  id: string;
  type: "success" | "error";
  title: ValueOrFunction<Renderable, ToastDefinition>;
  description: ValueOrFunction<Renderable, ToastDefinition>;
  duration?: number;
  createdAt: number;
};

export type ToastOptions = Partial<Pick<ToastDefinition, "id" | "duration">>;

export type ToastMessage = Pick<ToastDefinition, "title" | "description">;

export type ToastHandler = (message: ToastMessage, options?: ToastOptions) => string;

export type ToastController = {
  success: ToastHandler;
  error: ToastHandler;
  loading: ToastHandler;
};

export type ToastStore = {
  toasts: ToastDefinition[];
  add: (type: ToastDefinition["type"], message: ToastMessage, options?: ToastOptions) => string;
  remove: (id: string) => void;
};
