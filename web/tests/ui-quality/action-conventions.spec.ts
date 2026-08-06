import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";
import { expectButtonHasIcon } from "./helpers/layoutAssertions";

test.describe("Admin action conventions", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  test("common visible action buttons should use standard icons", async ({ page }) => {
    await page.goto("/buckets");

    await expectButtonHasIcon(page.getByTestId("create-bucket-button"), "create bucket button");
    await expectButtonHasIcon(page.getByRole("button", { name: /search|buscar/i }), "bucket search button");
    await expectButtonHasIcon(page.getByTestId("bucket-row").first().getByRole("button", { name: /delete bucket|excluir bucket/i }), "delete bucket button");

    await page.getByTestId("bucket-row").first().getByRole("button", { name: /delete bucket|excluir bucket/i }).click();
    await expectButtonHasIcon(page.getByTestId("confirm-dialog").getByRole("button", { name: /cancel|cancelar/i }), "confirmation cancel button");
    await expectButtonHasIcon(page.getByTestId("confirm-dialog").getByRole("button", { name: /confirm|confirmar/i }), "confirmation confirm button");
    await page.getByTestId("confirm-dialog-close").click();

    await page.getByTestId("bucket-row").first().getByRole("button", { name: /open|abrir/i }).click();
    await expectButtonHasIcon(page.getByTestId("hybrid-policy-save-button"), "hybrid policy save button");
  });

  test("destructive revoke actions should open confirmation dialogs before revoking", async ({ page }) => {
    await page.goto("/settings");

    await page.getByRole("button", { name: /revoke key|revogar chave/i }).first().click();
    await expect(page.getByTestId("confirm-dialog")).toBeVisible();
    await expect(page.getByText(/revoke s3 key|revogar chave s3/i)).toBeVisible();
    await page.getByTestId("confirm-dialog-close").click();

    await page.getByRole("button", { name: /revoke application|revogar aplicação/i }).first().click();
    await expect(page.getByTestId("confirm-dialog")).toBeVisible();
    await expect(page.getByText(/revoke application|revogar aplicação/i)).toBeVisible();
    await page.getByTestId("confirm-dialog-close").click();

    await page.getByRole("button", { name: /revoke token|revogar token/i }).first().click();
    await expect(page.getByTestId("confirm-dialog")).toBeVisible();
    await expect(page.getByText(/revoke mcp token|revogar token mcp/i)).toBeVisible();
    await page.getByTestId("confirm-dialog-close").click();

    await page.goto("/replicas");
    await page.getByRole("button", { name: /revoke replica|revogar réplica/i }).first().click();
    await expect(page.getByTestId("confirm-dialog")).toBeVisible();
    await expect(page.getByText(/revoke replica|revogar réplica/i)).toBeVisible();
  });
});
