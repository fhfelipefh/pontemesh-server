import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";

test.describe("Basic accessibility quality", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  for (const route of ["/dashboard", "/buckets", "/objects", "/settings"]) {
    test(`${route} should not have serious or critical accessibility violations`, async ({ page }) => {
      await page.goto(route);

      const results = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa"])
        .disableRules(["color-contrast"])
        .analyze();

      const serious = results.violations.filter((violation) =>
        ["serious", "critical"].includes(violation.impact ?? "")
      );

      expect(serious).toEqual([]);
    });
  }
});
