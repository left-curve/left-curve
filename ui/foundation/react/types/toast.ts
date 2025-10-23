import type { ValueOrFunction } from "@left-curve/dango/types";
import type { Renderable } from "./react";

export type ToastDefinition = {
  id: string;
  type: "info" | "success" | "error" | "maintenance";
  title: ValueOrFunction<Renderable, ToastDefinition>;
  description: ValueOrFunction<Renderable, ToastDefinition>;
  duration?: number;
  createdAt: number;
};

export type ToastOptions = Partial<Pick<ToastDefinition, "id" | "duration">>;

export type ToastMessage = Pick<ToastDefinition, "title" | "description">;

export type ToastHandler = (message: ToastMessage, options?: ToastOptions) => string;

export type ToastController = {
  info: ToastHandler;
  success: ToastHandler;
  error: ToastHandler;
  maintenance: ToastHandler;
  dismiss: (id: string) => void;
};

export type ToastStore = {
  toasts: ToastDefinition[];
  add: (type: ToastDefinition["type"], message: ToastMessage, options?: ToastOptions) => string;
  remove: (id: string) => void;
};
