import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";

test("an administrator can rename the instance and see it immediately", async ({ page }) => {
  await installAdminApiFixtures(page);
  await page.goto("/settings");

  const input = page.getByTestId("instance-name-input");
  await expect(input).toHaveValue("Ponte Mesh QA");
  await input.fill("Local Game Origin");
  await page.getByTestId("save-instance-name").click();

  await expect(page.locator(".admin-topbar strong")).toHaveText("Local Game Origin");
  await expect(input).toHaveValue("Local Game Origin");
});
