import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { HeaderExplorer } from "../src/components/explorer/HeaderExplorer";

describe("shared explorer shell", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders the explorer map artwork while preserving caller-provided content", () => {
    render(
      <HeaderExplorer>
        <h1>Missing account</h1>
        <p>0x6d697373696e6700000000000000000000000000</p>
      </HeaderExplorer>,
    );

    expect(screen.getByAltText("map-emoji")).toHaveAttribute(
      "src",
      "/images/emojis/simple/map.svg",
    );
    expect(screen.getByRole("heading", { name: "Missing account" })).toBeInTheDocument();
    expect(
      screen.getByText("0x6d697373696e6700000000000000000000000000"),
    ).toBeInTheDocument();
  });
});
