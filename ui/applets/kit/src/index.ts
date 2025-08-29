export * from "@left-curve/foundation-shared";
export * from "./components";

/* -------------------------------------------------------------------------- */
/*                                    Hooks                                   */
/* -------------------------------------------------------------------------- */

export {
  useTheme,
  type UseThemeReturnType,
  type Themes,
  type ThemesSchema,
} from "./hooks/useTheme";

export { useMediaQuery } from "./hooks/useMediaQuery";
export { useDOMRef } from "./hooks/useDOMRef";
export { useClickAway } from "./hooks/useClickAway";
export { useHasMounted } from "./hooks/useHasMounted";
export { usePortalTarget } from "./hooks/usePortalTarget";
