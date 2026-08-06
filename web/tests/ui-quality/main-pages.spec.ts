import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";
import {
  expectButtonsHaveUsableSize,
  expectNoHorizontalOverflow,
  expectNoTinyTextInputs
} from "./helpers/layoutAssertions";

const routes = ["/dashboard", "/buckets", "/objects", "/settings", "/metrics", "/replicas"];

test.describe("UI quality checks for main admin pages", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  for (const route of routes) {
    test(`${route} should not have basic layout regressions`, async ({ page }) => {
      await page.goto(route);

      await expect(page.locator("#root")).not.toBeEmpty();
      await expect(page.locator("main.admin-content")).toBeVisible();

      await expectNoHorizontalOverflow(page);
      await expectNoTinyTextInputs(page);
      await expectButtonsHaveUsableSize(page);
    });
  }
});
