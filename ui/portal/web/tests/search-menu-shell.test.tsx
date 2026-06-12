import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitTextLoopFactory,
  setAppletsKitUseAnimateOnceFactory,
  setAppletsKitUseAppFactory,
  setAppletsKitUseClickAwayFactory,
  setAppletsKitUseMediaQueryFactory,
  setAppletsKitUsePreserveScrollFactory,
} from "./mocks/applets-kit";
import { APPLETS } from "../constants.config";
import { SearchMenu } from "../src/components/foundation/SearchMenu";

const searchMenuShellMocks = vi.hoisted(() => ({
  clickAwayCallback: undefined as (() => void) | undefined,
  favApplets: ["trade", "transfer"] as string[],
  isLg: true,
  isSearchBarVisible: false,
  locationPathname: "/",
  navigate: vi.fn(),
  searchText: "",
  setSearchBarVisibility: vi.fn(),
  setSearchText: vi.fn(),
  useSearchBar: vi.fn(),
}));

function omitMotionProps<T extends Record<string, unknown>>(props: T) {
  const {
    animate: _animate,
    custom: _custom,
    exit: _exit,
    initial: _initial,
    layout: _layout,
    layoutId: _layoutId,
    layoutRoot: _layoutRoot,
    transition: _transition,
    variants: _variants,
    ...domProps
  } = props;

  return domProps;
}

vi.mock("framer-motion", async () => {
  const React = await import("react");

  const MotionDiv = React.forwardRef<
    HTMLDivElement,
    React.PropsWithChildren<React.HTMLAttributes<HTMLDivElement> & Record<string, unknown>>
  >(({ children, ...props }, ref) =>
    React.createElement("div", { ...omitMotionProps(props), ref }, children),
  );
  const MotionButton = React.forwardRef<
    HTMLButtonElement,
    React.PropsWithChildren<React.ButtonHTMLAttributes<HTMLButtonElement> & Record<string, unknown>>
  >(({ children, ...props }, ref) =>
    React.createElement("button", { ...omitMotionProps(props), ref }, children),
  );

  MotionDiv.displayName = "MotionDiv";
  MotionButton.displayName = "MotionButton";

  return {
    AnimatePresence: ({ children }: React.PropsWithChildren) =>
      React.createElement(React.Fragment, null, children),
    motion: {
      button: MotionButton,
      div: MotionDiv,
    },
  };
});

vi.mock("cmdk", async () => {
  const React = await import("react");

  type CommandRootProps = React.PropsWithChildren<
    React.HTMLAttributes<HTMLDivElement> & { shouldFilter?: boolean }
  >;
  type CommandInputProps = React.InputHTMLAttributes<HTMLInputElement> & {
    onValueChange?: (value: string) => void;
  };

  const CommandRoot = React.forwardRef<HTMLDivElement, CommandRootProps>(
    ({ children, shouldFilter: _shouldFilter, ...props }, ref) =>
      React.createElement("div", { ...props, ref }, children),
  );
  const CommandInput = React.forwardRef<HTMLInputElement, CommandInputProps>(
    ({ onChange, onValueChange, ...props }, ref) =>
      React.createElement("input", {
        ...props,
        ref,
        onChange: (event: React.ChangeEvent<HTMLInputElement>) => {
          onValueChange?.(event.target.value);
          onChange?.(event);
        },
      }),
  );

  CommandRoot.displayName = "CommandRoot";
  CommandInput.displayName = "CommandInput";

  return {
    Command: Object.assign(CommandRoot, {
      Empty: ({ children }: React.PropsWithChildren) => React.createElement("div", null, children),
      Group: ({ children, heading }: React.PropsWithChildren<{ heading: string }>) =>
        React.createElement("section", { "aria-label": heading }, children),
      Input: CommandInput,
      Item: ({
        children,
        onSelect,
        value,
      }: React.PropsWithChildren<{ onSelect?: () => void; value?: string }>) =>
        React.createElement("div", { "data-value": value, onClick: onSelect }, children),
      List: ({ children }: React.PropsWithChildren) => React.createElement("div", null, children),
    }),
  };
});

vi.mock("@tanstack/react-router", () => ({
  useLocation: () => ({ pathname: searchMenuShellMocks.locationPathname }),
  useNavigate: () => searchMenuShellMocks.navigate,
}));

vi.mock("@left-curve/store", () => ({
  useFavApplets: () => ({
    favApplets: searchMenuShellMocks.favApplets,
  }),
  useSearchBar: (params: unknown) => {
    searchMenuShellMocks.useSearchBar(params);

    return {
      allNotFavApplets: [],
      isLoading: false,
      isRefetching: false,
      searchResult: {
        applets: [],
        contracts: [],
        txs: [],
      },
      searchText: searchMenuShellMocks.searchText,
      setSearchText: searchMenuShellMocks.setSearchText,
    };
  },
}));

function renderSearchMenu() {
  return render(<SearchMenu />);
}

describe("SearchMenu shell", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitTextLoopFactory(({ texts }) => <span>{texts[0]}</span>);
    setAppletsKitUseAnimateOnceFactory(() => true);
    setAppletsKitUseAppFactory(() => ({
      isSearchBarVisible: searchMenuShellMocks.isSearchBarVisible,
      setSearchBarVisibility: searchMenuShellMocks.setSearchBarVisibility,
    }));
    setAppletsKitUseClickAwayFactory((_ref, callback) => {
      searchMenuShellMocks.clickAwayCallback = callback as () => void;
    });
    setAppletsKitUseMediaQueryFactory(() => ({
      isLg: searchMenuShellMocks.isLg,
      isMd: true,
    }));
    setAppletsKitUsePreserveScrollFactory(() => ({ current: null }));
    searchMenuShellMocks.clickAwayCallback = undefined;
    searchMenuShellMocks.favApplets = ["trade", "transfer"];
    searchMenuShellMocks.isLg = true;
    searchMenuShellMocks.isSearchBarVisible = false;
    searchMenuShellMocks.locationPathname = "/";
    searchMenuShellMocks.searchText = "";
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("wires favorites and app metadata into the search hook and opens from the placeholder", () => {
    renderSearchMenu();

    expect(searchMenuShellMocks.useSearchBar).toHaveBeenCalledWith({
      applets: APPLETS,
      favApplets: ["trade", "transfer"],
    });
    expect(
      screen.getByLabelText(
        `${m["searchBar.placeholder.title"]()} ${m["searchBar.placeholder.apps"]()}`,
      ),
    ).toBeInTheDocument();

    const placeholderButton = screen
      .getByText(m["searchBar.placeholder.title"]())
      .closest("button");
    if (!(placeholderButton instanceof HTMLButtonElement)) {
      throw new Error("Expected search placeholder button");
    }

    fireEvent.click(placeholderButton);

    expect(searchMenuShellMocks.setSearchBarVisibility).toHaveBeenCalledWith(true);
  });

  it("uses desktop keyboard shortcuts to open, type, and close the search menu", () => {
    const { unmount } = renderSearchMenu();
    const input = screen.getByLabelText(
      `${m["searchBar.placeholder.title"]()} ${m["searchBar.placeholder.apps"]()}`,
    );

    fireEvent.keyDown(window, { key: "k", metaKey: true });

    expect(searchMenuShellMocks.setSearchBarVisibility).toHaveBeenCalledWith(true);
    expect(document.activeElement).toBe(input);

    unmount();
    cleanup();
    vi.clearAllMocks();

    renderSearchMenu();
    fireEvent.keyDown(window, { key: "x" });

    expect(searchMenuShellMocks.setSearchBarVisibility).toHaveBeenCalledWith(true);
    const appendCall = searchMenuShellMocks.setSearchText.mock.calls.find(
      ([value]) => typeof value === "function",
    );
    expect(appendCall).toBeDefined();
    expect((appendCall![0] as (current: string) => string)("bt")).toBe("btx");

    cleanup();
    vi.clearAllMocks();
    searchMenuShellMocks.isSearchBarVisible = true;
    renderSearchMenu();

    fireEvent.keyDown(window, { key: "Escape" });

    expect(searchMenuShellMocks.setSearchBarVisibility).toHaveBeenCalledWith(false);
    expect(searchMenuShellMocks.setSearchText).toHaveBeenCalledWith("");
  });

  it("hides and clears on desktop click-away but keeps mobile search open", () => {
    renderSearchMenu();

    searchMenuShellMocks.clickAwayCallback?.();

    expect(searchMenuShellMocks.setSearchBarVisibility).toHaveBeenCalledWith(false);
    expect(searchMenuShellMocks.setSearchText).toHaveBeenCalledWith("");

    cleanup();
    vi.clearAllMocks();
    searchMenuShellMocks.isLg = false;
    renderSearchMenu();

    searchMenuShellMocks.clickAwayCallback?.();

    expect(searchMenuShellMocks.setSearchBarVisibility).not.toHaveBeenCalled();
    expect(searchMenuShellMocks.setSearchText).not.toHaveBeenCalled();
  });
});
