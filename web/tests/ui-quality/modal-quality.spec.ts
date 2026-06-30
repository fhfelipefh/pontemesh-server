import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";

test.describe("Modal and drawer quality", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  test("bucket drawer should keep backdrop and dialog visually stable", async ({ page }) => {
    await openBucketsPage(page);
    await page.getByTestId("bucket-row").first().getByRole("button", { name: /open|abrir/i }).click();

    const backdrop = page.getByTestId("modal-backdrop");
    const dialog = page.getByTestId("bucket-details-dialog");

    await expect(backdrop).toHaveCount(1);
    await expect(backdrop).toBeVisible();
    await expect(dialog).toBeVisible();

    const firstBox = await dialog.boundingBox();
    await page.waitForTimeout(300);
    const secondBox = await dialog.boundingBox();

    expect(firstBox, "dialog should be measurable after opening").not.toBeNull();
    expect(secondBox, "dialog should remain measurable after settling").not.toBeNull();

    if (firstBox && secondBox) {
      expect(Math.abs(firstBox.x - secondBox.x), "dialog x position should not flicker").toBeLessThanOrEqual(1);
      expect(Math.abs(firstBox.y - secondBox.y), "dialog y position should not flicker").toBeLessThanOrEqual(1);
      expect(Math.abs(firstBox.width - secondBox.width), "dialog width should not flicker").toBeLessThanOrEqual(1);
      expect(Math.abs(firstBox.height - secondBox.height), "dialog height should not flicker").toBeLessThanOrEqual(1);
    }

    const drawerOwnsCenterPoint = await page.evaluate(() => {
      const dialog = document.querySelector("[data-testid='bucket-details-dialog']");
      if (!dialog) {
        return false;
      }
      const box = dialog.getBoundingClientRect();
      const element = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
      return Boolean(element?.closest("[data-testid='bucket-details-dialog']"));
    });

    expect(drawerOwnsCenterPoint, "drawer should be the top element at its center point").toBe(true);
  });

  test("content behind drawer should not receive pointer interaction", async ({ page }) => {
    await openBucketsPage(page);
    const clicksBefore = await page.getByTestId("modal-backdrop").count();
    await page.getByTestId("bucket-row").first().getByRole("button", { name: /open|abrir/i }).click();

    await expect(page.getByTestId("bucket-details-dialog")).toBeVisible();

    const createButtonBlocked = await page.evaluate(() => {
      const button = document.querySelector("[data-testid='create-bucket-button']");
      if (!button) {
        return false;
      }
      const box = button.getBoundingClientRect();
      const element = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
      return element !== button && !button.contains(element);
    });

    expect(clicksBefore).toBe(0);
    expect(createButtonBlocked, "page action behind drawer should be covered by backdrop").toBe(true);
  });

  test("create bucket modal should trap initial focus and close through Escape", async ({ page }) => {
    await openBucketsPage(page);
    await page.getByTestId("create-bucket-button").click();

    const dialog = page.getByRole("dialog", { name: /create bucket|criar bucket/i });
    await expect(dialog).toBeVisible();
    await expect(page.getByTestId("modal-backdrop")).toHaveCount(1);
    await expect(page.locator("#bucket-name")).toBeFocused();

    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
  });

  test("close icons should close each bucket overlay without leaving stale backdrops", async ({ page }) => {
    await openBucketsPage(page);
    await expectNoVisibleOverlays(page);

    await page.getByTestId("create-bucket-button").click();
    await expect(page.getByTestId("create-bucket-dialog")).toBeVisible();
    await expect(page.getByTestId("modal-backdrop")).toHaveCount(1);
    await page.getByTestId("create-bucket-close").click();
    await expectNoVisibleOverlays(page);

    await openFirstBucketDrawer(page);
    await expect(page.getByTestId("bucket-details-dialog")).toBeVisible();
    await expect(page.getByTestId("modal-backdrop")).toHaveCount(1);
    await page.getByTestId("bucket-details-close").click();
    await expectNoVisibleOverlays(page);

    await page.getByTestId("bucket-row").first().getByRole("button", { name: /delete bucket|excluir bucket/i }).click();
    await expect(page.getByTestId("confirm-dialog")).toBeVisible();
    await expect(page.getByTestId("modal-backdrop")).toHaveCount(1);
    await page.getByTestId("confirm-dialog-close").click();
    await expectNoVisibleOverlays(page);
  });

  test("opening bucket details should keep previously closed overlays closed", async ({ page }) => {
    await openBucketsPage(page);

    await page.getByTestId("create-bucket-button").click();
    await expect(page.getByTestId("create-bucket-dialog")).toBeVisible();
    await page.getByTestId("create-bucket-close").click();
    await expectNoVisibleOverlays(page);

    await page.getByTestId("bucket-row").first().getByRole("button", { name: /delete bucket|excluir bucket/i }).click();
    await expect(page.getByTestId("confirm-dialog")).toBeVisible();
    await page.getByTestId("confirm-dialog-close").click();
    await expectNoVisibleOverlays(page);

    await openFirstBucketDrawer(page);

    await expect(page.getByTestId("bucket-details-dialog")).toBeVisible();
    await expect(page.getByTestId("create-bucket-dialog")).toHaveCount(0);
    await expect(page.getByTestId("confirm-dialog")).toHaveCount(0);
    await expect(page.getByRole("dialog")).toHaveCount(1);
    await expect(page.getByTestId("modal-backdrop")).toHaveCount(1);
  });

  test("closing object delete confirmation should keep bucket drawer open and other overlays closed", async ({ page }) => {
    await openBucketsPage(page);
    await openFirstBucketDrawer(page);

    await page.getByTestId("bucket-details-dialog").getByRole("button", { name: /delete|excluir/i }).first().click();

    await expect(page.getByTestId("bucket-details-dialog")).toBeVisible();
    await expect(page.getByTestId("confirm-dialog")).toBeVisible();
    await expect(page.getByTestId("modal-backdrop")).toHaveCount(2);

    await page.getByTestId("confirm-dialog-close").click();

    await expect(page.getByTestId("confirm-dialog")).toHaveCount(0);
    await expect(page.getByTestId("create-bucket-dialog")).toHaveCount(0);
    await expect(page.getByTestId("bucket-details-dialog")).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(1);
    await expect(page.getByTestId("modal-backdrop")).toHaveCount(1);
  });
});

async function openBucketsPage(page: import("@playwright/test").Page) {
  await page.goto("/dashboard");
  await page.getByTestId("app-sidebar").getByRole("link", { name: /buckets/i }).click();
  await expect(page).toHaveURL(/\/buckets$/);
}

async function openFirstBucketDrawer(page: import("@playwright/test").Page) {
  await page.getByTestId("bucket-row").first().getByRole("button", { name: /open|abrir/i }).click();
  await expect(page.getByTestId("bucket-details-dialog")).toBeVisible();
}

async function expectNoVisibleOverlays(page: import("@playwright/test").Page) {
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByTestId("modal-backdrop")).toHaveCount(0);
}
