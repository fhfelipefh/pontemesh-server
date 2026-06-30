import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";

test.describe("Sidebar quality", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  test("sidebar should collapse, expand and keep version fixed at bottom", async ({ page }) => {
    await page.goto("/dashboard");
    await page.evaluate(() => localStorage.removeItem("pontemesh.sidebarCollapsed"));
    await page.reload();

    const sidebar = page.getByTestId("app-sidebar");
    const toggle = page.getByTestId("sidebar-toggle");
    const version = page.getByTestId("sidebar-version");

    await expect(sidebar).toBeVisible();
    await expect(version).toBeVisible();

    const expandedBox = await sidebar.boundingBox();
    expect(expandedBox?.width).toBeGreaterThanOrEqual(220);

    await toggle.click();

    const collapsedBox = await sidebar.boundingBox();
    expect(collapsedBox?.width).toBeLessThanOrEqual(90);

    const versionBox = await version.boundingBox();
    const sidebarBox = await sidebar.boundingBox();

    expect(versionBox).not.toBeNull();
    expect(sidebarBox).not.toBeNull();

    if (versionBox && sidebarBox) {
      expect(versionBox.y + versionBox.height).toBeLessThanOrEqual(sidebarBox.y + sidebarBox.height);
      expect(versionBox.y).toBeGreaterThan(sidebarBox.y + sidebarBox.height - 96);
    }

    await page.reload();

    const persistedBox = await sidebar.boundingBox();
    expect(persistedBox?.width).toBeLessThanOrEqual(90);

    await toggle.click();
    const reexpandedBox = await sidebar.boundingBox();
    expect(reexpandedBox?.width).toBeGreaterThanOrEqual(220);
  });
});
