import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("identifies the application as fully offline decision support", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { name: "CHEW Companion" }),
    ).toBeVisible();
    expect(
      screen.getByText("Fully offline clinical decision support"),
    ).toBeVisible();
    expect(
      screen.getByText(/does not replace clinical judgment/i),
    ).toBeVisible();
  });

  it("loads the supported cadres from the runtime contract", async () => {
    render(<App />);

    expect(
      await screen.findByText("Supported cadres: JCHEW + CHEW"),
    ).toBeVisible();
  });
});
