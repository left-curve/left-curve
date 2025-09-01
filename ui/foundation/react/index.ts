/* -------------------------------------------------------------------------- */
/*                                    Hooks                                   */
/* -------------------------------------------------------------------------- */

export { usePagination, type UsePaginationParameters } from "./hooks/usePagination.js";
export { useControlledState } from "./hooks/useControlledState.js";
export { useCountdown } from "./hooks/useCountdown.js";
export { useInputs } from "./hooks/useInputs.js";
export { useWatchEffect } from "./hooks/useWatch.js";

/* -------------------------------------------------------------------------- */
/*                                  Providers                                 */
/* -------------------------------------------------------------------------- */

export { WizardProvider, useWizard } from "./providers/WizardProvider.js";

/* -------------------------------------------------------------------------- */
/*                                    Types                                   */
/* -------------------------------------------------------------------------- */

export type { AppletMetadata } from "./types/applets.js";
export type { PolymorphicComponent, PolymorphicRenderFunction } from "./types/polymorph.js";
export type { AbstractStorage, CreateStorageParameters, Storage } from "./types/storage.js";

/* -------------------------------------------------------------------------- */
/*                                    Utils                                   */
/* -------------------------------------------------------------------------- */

export {
  createContext,
  type CreateContextOptions,
  type CreateContextReturn,
} from "./utils/context.js";

export { twMerge } from "./utils/twMerge.js";
export { mergeRefs } from "./utils/mergeRefs.js";
export { forwardRefPolymorphic } from "./utils/polymorph.js";
export { numberMask } from "./utils/masks.js";
export { ensureErrorMessage } from "./utils/error.js";
