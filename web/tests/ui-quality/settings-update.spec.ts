import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";

test("confirms a server update before it is requested", async ({ page }) => {
  await installAdminApiFixtures(page);
  await page.goto("/settings");

  await expect(page.getByText("A new Ponte Mesh Server version is available: 0.3.4.")).toBeVisible();
  await page.getByTestId("request-server-update").click();
  await expect(page.getByTestId("confirm-dialog")).toContainText("the service will restart when it finishes.");
  await page.getByTestId("confirm-dialog").getByRole("button", { name: "Update now" }).click();
  await expect(page.getByText("Update started. The service will restart when it finishes.")).toBeVisible();
});
