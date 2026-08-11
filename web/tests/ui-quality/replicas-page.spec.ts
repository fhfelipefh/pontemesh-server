import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";
import { expectNoHorizontalOverflow } from "./helpers/layoutAssertions";

test.describe("Replicas page", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  test("shows replica health, scope, and creation controls in a responsive card layout", async ({ page }) => {
    await page.goto("/replicas");

    await expect(page.locator("h1")).toHaveText(/replicas|réplicas/i);
    await expect(page.getByLabel(/replica summary|resumo das réplicas/i)).toBeVisible();
    await expect(page.getByLabel(/replica name|nome da réplica/i)).toBeVisible();
    await expect(page.getByRole("textbox", { name: /allowed buckets|buckets permitidos/i })).toBeVisible();

    const card = page.locator(".replica-card");
    await expect(card).toHaveCount(1);
    await expect(card.getByText("Replica QA", { exact: true })).toBeVisible();
    await expect(card.getByText("assets", { exact: true })).toBeVisible();
    await expect(card.getByRole("button", { name: /revoke replica|revogar réplica/i })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });
});
