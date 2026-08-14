import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { CopyButton } from "./CopyButton";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key
  })
}));

describe("CopyButton", () => {
  it("renders button with initial label and accessible attributes", () => {
    const markup = renderToStaticMarkup(<CopyButton value="test-value" label="Copy token" />);

    expect(markup).toContain('title="Copy token"');
    expect(markup).toContain('aria-label="Copy token"');
    expect(markup).toContain("settings-icon-button");
  });
});
