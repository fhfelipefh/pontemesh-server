import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";
import {
  expectBodyDoesNotScroll,
  expectElementMaxSize,
  expectElementMinSize,
  expectNoHorizontalOverflow,
  UI_LIMITS
} from "./helpers/layoutAssertions";

test.describe("Buckets page layout quality", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  test("bucket search and create button should have usable desktop proportions", async ({ page }) => {
    await openBucketsPage(page);

    const searchInput = page.getByTestId("bucket-search-input");
    await expect(searchInput).toBeVisible();
    await expectElementMinSize(searchInput, {
      label: "bucket search input",
      minWidth: UI_LIMITS.searchInputMinWidth,
      minHeight: 38
    });

    const createButton = page.getByTestId("create-bucket-button");
    await expect(createButton).toBeVisible();
    await expectElementMinSize(createButton, {
      label: "create bucket button",
      minWidth: 120,
      minHeight: 38
    });
    await expectElementMaxSize(createButton, {
      label: "create bucket button",
      maxWidth: UI_LIMITS.primaryButtonMaxWidthDesktop
    });

    await expect(page.getByTestId("bucket-list")).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await expectBodyDoesNotScroll(page);
  });

  test("bucket details should open in a drawer above the page without expanding body scroll", async ({ page }) => {
    await openBucketsPage(page);

    await page.getByTestId("bucket-row").first().getByRole("button", { name: /open|abrir/i }).click();

    const dialog = page.getByTestId("bucket-details-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute("aria-modal", "true");
    await expect(page.getByTestId("modal-backdrop")).toHaveCount(1);

    await expectNoHorizontalOverflow(page);
    await expectBodyDoesNotScroll(page);
  });

  test("object upload should reset the form without throwing runtime errors", async ({ page }) => {
    const pageErrors: string[] = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));

    await openBucketsPage(page);
    await page.getByTestId("bucket-row").first().getByRole("button", { name: /open|abrir/i }).click();

    await page.getByTestId("open-upload-object-button").click();
    await expect(page.getByTestId("upload-object-dialog")).toBeVisible();
    await page.getByTestId("object-file-input").setInputFiles({
      name: "upload-check.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("pontemesh upload check")
    });

    await expect(page.getByTestId("upload-object-button")).toBeEnabled();
    await page.getByTestId("upload-object-button").click();

    await expect(page.getByTestId("upload-object-dialog")).toBeHidden();
    expect(pageErrors).toEqual([]);
  });
});

async function openBucketsPage(page: import("@playwright/test").Page) {
  await page.goto("/dashboard");
  await page.getByTestId("app-sidebar").getByRole("link", { name: /buckets/i }).click();
  await expect(page).toHaveURL(/\/buckets$/);
}
