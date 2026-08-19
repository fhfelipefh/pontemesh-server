import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";

test("confirms a server update before it is requested", async ({ page }) => {
  await installAdminApiFixtures(page);
  await page.goto("/settings");

  await expect(page.getByText("A new Ponte Mesh Server version is available: 0.3.4.")).toBeVisible();
  await expect(page.getByText("Automatic updates are not configured.")).toHaveCount(0);
  await expect(page.getByTestId("request-server-update")).toBeEnabled();
  await page.getByTestId("request-server-update").click();
  await expect(page.getByTestId("confirm-dialog")).toContainText("the service will restart when it finishes.");
  await page.getByTestId("confirm-dialog").getByRole("button", { name: "Update now" }).click();
  await expect(page.getByText("Update started. The service will restart when it finishes.")).toBeVisible();
});

test("shows and saves the maximum storage capacity setting", async ({ page }) => {
  await installAdminApiFixtures(page);
  await page.goto("/settings");

  await expect(page.getByRole("heading", { name: "Storage capacity" })).toBeVisible();
  await expect(page.getByLabel("Maximum usage limit")).toHaveValue("95");
  await page.getByRole("switch", { name: "Enforce capacity limit" }).click();
  await expect(page.getByLabel("Maximum usage limit")).toBeHidden();
  await page.getByRole("switch", { name: "Enforce capacity limit" }).click();
  await expect(page.getByLabel("Maximum usage limit")).toBeVisible();
  await page.getByLabel("Maximum usage limit").fill("97");
  await page.getByTestId("save-storage-capacity").click();

  await expect(page.getByText("Saved", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Maximum usage limit")).toHaveValue("97");
});

test("exposes administrator accounts as a named semantic list", async ({ page }) => {
  await installAdminApiFixtures(page);
  await page.goto("/users");

  const accounts = page.getByRole("table");
  await expect(accounts).toBeVisible();
  await expect(accounts.getByRole("cell", { name: "admin", exact: true })).toBeVisible();
});

test("configures an operational webhook and keeps its JSON preview collapsed", async ({ page }) => {
  await installAdminApiFixtures(page);
  await page.goto("/settings");

  const card = page.getByRole("heading", { name: "Operational webhook" }).locator("xpath=ancestor::section");
  await expect(card.getByLabel("URL")).toBeHidden();
  await expect(card.getByText("JSON object sent")).toBeHidden();
  await card.getByRole("switch", { name: "Send operational webhook" }).click();
  await expect(card.getByLabel("URL")).toBeVisible();
  await expect(card.getByText('"event": "pontemesh.operational_status"')).toBeHidden();
  await card.getByText("JSON object sent").click();
  await expect(card.getByText('"event": "pontemesh.operational_status"')).toBeVisible();
  await card.getByLabel("URL").fill("http://localhost:5678/webhook/storage");
  await card.getByLabel("Cron schedule").fill("*/5 * * * *");
  await card.getByTestId("save-operational-webhook").click();

  await expect(card.getByText("Saved", { exact: true })).toBeVisible();
  await expect(card.getByLabel("URL")).toHaveValue("http://localhost:5678/webhook/storage");
  await expect(card.getByLabel("Cron schedule")).toHaveValue("*/5 * * * *");
});

test("creates an administrator only after a strong password and explicit confirmation", async ({ page }) => {
  await installAdminApiFixtures(page);
  await page.goto("/users");

  const createForm = page.locator("form").first();
  const createButton = createForm.getByRole("button", { name: "Create user" });
  await createForm.getByLabel("Username").fill("operations-admin");
  await createForm.getByLabel("Password", { exact: true }).fill("onlylowercase123");
  await createForm.getByLabel("Your current password").fill("CurrentAdmin123!");
  await expect(createButton).toBeDisabled();
  await createForm.getByLabel("Password", { exact: true }).fill("PonteMeshAdmin123!");
  await expect(createButton).toBeEnabled();
  await createButton.click();

  await expect(page.getByRole("table").getByRole("cell", { name: "operations-admin", exact: true })).toBeVisible();
});
