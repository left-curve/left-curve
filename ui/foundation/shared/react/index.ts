/* -------------------------------------------------------------------------- */
/*                                    Hooks                                   */
/* -------------------------------------------------------------------------- */

export { usePagination, type UsePaginationParameters } from "./hooks/usePagination";
export { useControlledState } from "./hooks/useControlledState";
export { useCountdown } from "./hooks/useCountdown";
export { useInputs } from "./hooks/useInputs";
export { useWatchEffect } from "./hooks/useWatch";
export { useStorage, type UseStorageOptions } from "./hooks/useStorage";

/* -------------------------------------------------------------------------- */
/*                                  Providers                                 */
/* -------------------------------------------------------------------------- */

export { WizardProvider, useWizard } from "./providers/WizardProvider";

/* -------------------------------------------------------------------------- */
/*                                   Storage                                  */
/* -------------------------------------------------------------------------- */

export { createMemoryStorage } from "./storages/memoryStorage.js";
export { createStorage } from "./storages/createStorage.js";

/* -------------------------------------------------------------------------- */
/*                                    Types                                   */
/* -------------------------------------------------------------------------- */

export type { AppletMetadata } from "./types/applets";
export type { PolymorphicComponent, PolymorphicRenderFunction } from "./types/polymorph";
export type { AbstractStorage, CreateStorageParameters, Storage } from "./types/storage";

/* -------------------------------------------------------------------------- */
/*                                    Utils                                   */
/* -------------------------------------------------------------------------- */

export {
  createContext,
  type CreateContextOptions,
  type CreateContextReturn,
} from "./utils/context";

export { twMerge } from "./utils/twMerge";
export { mergeRefs } from "./utils/mergeRefs";
export { forwardRefPolymorphic } from "./utils/polymorph";
export { numberMask } from "./utils/masks";
export { ensureErrorMessage } from "./utils/error";
