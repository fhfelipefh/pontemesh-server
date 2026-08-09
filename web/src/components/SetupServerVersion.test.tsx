import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SetupServerVersion } from "./SetupServerVersion";

describe("SetupServerVersion", () => {
  it("renders only the server version with a discreet version prefix", () => {
    const markup = renderToStaticMarkup(<SetupServerVersion version="0.2.2" />);

    expect(markup).toContain("v0.2.2");
    expect(markup).not.toContain("Ponte Mesh Server");
    expect(markup).toContain('class="setup-page__version"');
  });
});
