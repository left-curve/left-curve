import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { CHARACTERS, CharacterSelector } from "../src/components/foundation/CharacterSelector";

describe("CharacterSelector", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders every configured character thumbnail and marks the selected option", () => {
    render(<CharacterSelector selected={3} onSelect={vi.fn()} />);

    expect(screen.getByText(m["modals.shareCard.overlay"]())).toBeInTheDocument();
    expect(screen.getAllByRole("button")).toHaveLength(CHARACTERS.length);

    for (const character of CHARACTERS) {
      expect(screen.getByAltText(character)).toHaveAttribute(
        "src",
        `/images/pnl-modal-thumb/${character}.png`,
      );
    }

    const selectedButton = screen.getByRole("button", { name: CHARACTERS[3] });
    expect(selectedButton).toHaveClass("border-primitives-red-light-500");
    expect(selectedButton.querySelector("svg")).not.toBeNull();

    const unselectedButton = screen.getByRole("button", { name: CHARACTERS[0] });
    expect(unselectedButton).toHaveClass("border-outline-secondary-gray");
    expect(unselectedButton.querySelector("svg")).toBeNull();
  });

  it("selects thumbnails by their exported character index", () => {
    const onSelect = vi.fn();

    render(<CharacterSelector selected={0} onSelect={onSelect} />);

    fireEvent.click(screen.getByRole("button", { name: CHARACTERS[5] }));
    fireEvent.click(screen.getByRole("button", { name: CHARACTERS[12] }));

    expect(onSelect).toHaveBeenNthCalledWith(1, 5);
    expect(onSelect).toHaveBeenNthCalledWith(2, 12);
  });
});
