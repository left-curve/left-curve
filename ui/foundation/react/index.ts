/* -------------------------------------------------------------------------- */
/*                                    Hooks                                   */
/* -------------------------------------------------------------------------- */

export { usePagination, type UsePaginationParameters } from "./hooks/usePagination.js";
export { useControlledState } from "./hooks/useControlledState.js";
export { useCountdown } from "./hooks/useCountdown.js";
export { useInputs, type Controllers } from "./hooks/useInputs.js";
export { useWatchEffect } from "./hooks/useWatch.js";

/* -------------------------------------------------------------------------- */
/*                                  Providers                                 */
/* -------------------------------------------------------------------------- */

export { WizardProvider, useWizard } from "./providers/WizardProvider.js";

export {
  AppProvider,
  useApp,
  type AppProviderProps,
  type AppState,
} from "./providers/AppProvider.js";

/* -------------------------------------------------------------------------- */
/*                                    Types                                   */
/* -------------------------------------------------------------------------- */

export type { PolymorphicComponent, PolymorphicRenderFunction } from "./types/polymorph.js";
export type {
  ToastDefinition,
  ToastMessage,
  ToastOptions,
  ToastHandler,
  ToastController,
  ToastStore,
} from "./types/toast.js";
export { Modals, type ModalRef, type ModalDefinition } from "./types/modals.js";
export type { Renderable } from "./types/react.js";

/* -------------------------------------------------------------------------- */
/*                                    Utils                                   */
/* -------------------------------------------------------------------------- */

export {
  createContext,
  type CreateContextOptions,
  type CreateContextReturn,
} from "./utils/context.js";

export { formatDate, formatActivityTimestamp } from "./utils/dates.js";
export { twMerge } from "./utils/twMerge.js";
export { mergeRefs } from "./utils/mergeRefs.js";
export { forwardRefPolymorphic } from "./utils/polymorph.js";
export { numberMask } from "./utils/masks.js";
export { ensureErrorMessage } from "./utils/error.js";
