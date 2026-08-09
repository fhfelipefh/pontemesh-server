import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";

test("an administrator can rename the instance and see it immediately", async ({ page }) => {
  await installAdminApiFixtures(page);
  await page.goto("/settings");

  const input = page.getByTestId("instance-name-input");
  await expect(input).toHaveValue("Ponte Mesh QA");
  const saveButton = page.getByTestId("save-instance-name");
  await expect(saveButton.locator("svg.lucide-save")).toBeVisible();
  await expect(page.getByText(/without altering|sem modificar o papel/i)).toHaveCount(0);
  await input.fill("Local Game Origin");
  await saveButton.click();

  await expect(page.locator(".admin-topbar strong")).toHaveText("Local Game Origin");
  await expect(input).toHaveValue("Local Game Origin");
});
