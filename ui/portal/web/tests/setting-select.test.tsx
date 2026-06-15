import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  resetAppletsKitMocks,
  setAppletsKitUseMediaQuery,
} from "./mocks/applets-kit";

import { SettingSelect } from "../src/components/settings/SettingSelect";

vi.mock("react-modal-sheet", async () => {
  const React = await import("react");

  const Sheet = Object.assign(
    ({ children, isOpen }: React.PropsWithChildren<{ isOpen: boolean }>) =>
      isOpen ? <div data-testid="mobile-sheet">{children}</div> : null,
    {
      Backdrop: ({ onTap }: { onTap?: () => void }) => (
        <button aria-label="close settings sheet" onClick={onTap} type="button" />
      ),
      Container: ({
        children,
        className: _className,
      }: React.PropsWithChildren<{ className?: string }>) => <section>{children}</section>,
      Content: ({ children }: React.PropsWithChildren) => <div>{children}</div>,
      Header: () => <div />,
    },
  );

  return { Sheet };
});

const options = [
  { value: "utc", label: "UTC" },
  { value: "local", label: "Local" },
];

function renderSettingSelect(onChange = vi.fn()) {
  render(
    <SettingSelect
      icon={<span data-testid="setting-icon" />}
      label="Time zone"
      onChange={onChange}
      options={options}
      value="utc"
    />,
  );

  return onChange;
}

describe("SettingSelect", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseMediaQuery({
      isMd: true,
    });
    Object.defineProperty(window, "scrollTo", {
      configurable: true,
      value: vi.fn(),
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("uses the desktop select and forwards selected values", () => {
    const onChange = renderSettingSelect();

    expect(screen.queryByText("Local")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("Time zone"));

    expect(screen.getByText("Local")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Local"));

    expect(onChange).toHaveBeenCalledWith("local");
  });

  it("opens the mobile sheet, selects an option, and closes it", () => {
    setAppletsKitUseMediaQuery({
      isMd: false,
    });
    const onChange = renderSettingSelect();

    expect(screen.getByText("UTC")).toBeInTheDocument();
    expect(screen.queryByTestId("mobile-sheet")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("Time zone"));

    expect(screen.getByTestId("mobile-sheet")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Time zone" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Local" }));

    expect(onChange).toHaveBeenCalledWith("local");
    expect(screen.queryByTestId("mobile-sheet")).not.toBeInTheDocument();
  });

  it("closes the mobile sheet from the backdrop without changing the setting", () => {
    setAppletsKitUseMediaQuery({
      isMd: false,
    });
    const onChange = renderSettingSelect();

    fireEvent.click(screen.getByText("Time zone"));
    fireEvent.click(screen.getByRole("button", { name: "close settings sheet" }));

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByTestId("mobile-sheet")).not.toBeInTheDocument();
  });
});
