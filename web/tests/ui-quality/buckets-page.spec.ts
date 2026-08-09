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

  test("bucket defaults and bulk settings should support selected or all buckets", async ({ page }) => {
    await openBucketsPage(page);

    const configureButton = page.getByTestId("bucket-policy-manager-button");
    const createButton = page.getByTestId("create-bucket-button");
    const configureBox = await configureButton.boundingBox();
    const createBox = await createButton.boundingBox();
    expect(configureBox).not.toBeNull();
    expect(createBox).not.toBeNull();
    if (configureBox && createBox) {
      expect(configureBox.x).toBeLessThan(createBox.x);
    }

    await page.getByRole("checkbox", { name: /select bucket assets|selecionar bucket assets/i }).check();
    await configureButton.click();

    const dialog = page.getByTestId("bucket-policy-manager-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel(/Max package TTL|TTL máximo do pacote/i)).toHaveValue("900");
    await expect(dialog.getByRole("radio", { name: /Selected buckets \(1\)|Buckets selecionados \(1\)/i })).toBeChecked();
    await expect(dialog.getByRole("radio", { name: /All buckets|Todos os buckets/i })).toBeEnabled();

    await dialog.getByLabel(/Max package TTL|TTL máximo do pacote/i).fill("1200");
    await dialog.getByRole("button", { name: /Save defaults|Salvar padrões/i }).click();
    await expect(dialog.getByText(/Defaults saved|Padrões salvos/i)).toBeVisible();

    await dialog.getByRole("button", { name: /Apply settings|Aplicar configurações/i }).click();
    await expect(dialog).toBeHidden();
    await expectNoHorizontalOverflow(page);
  });

  test("hybrid policy form should keep proportional controls and aligned checkboxes", async ({ page }) => {
    await openBucketsPage(page);

    await page.getByTestId("bucket-row").first().getByRole("button", { name: /open|abrir/i }).click();

    const section = page.getByTestId("hybrid-policy-section");
    await expect(section).toBeVisible();

    const saveButton = page.getByTestId("hybrid-policy-save-button");
    await expect(saveButton).toBeVisible();
    await expectElementMaxSize(saveButton, {
      label: "hybrid policy save button",
      maxWidth: 220
    });

    const buttonBox = await saveButton.boundingBox();
    const sectionBox = await section.boundingBox();
    expect(buttonBox, "hybrid policy save button should be measurable").not.toBeNull();
    expect(sectionBox, "hybrid policy section should be measurable").not.toBeNull();
    if (buttonBox && sectionBox) {
      expect(buttonBox.width, "hybrid policy save button should not occupy the full section width").toBeLessThan(sectionBox.width * 0.5);
    }

    const fieldControls = section.locator(".form-field input, .form-field select");
    const fieldCount = await fieldControls.count();
    expect(fieldCount, "hybrid policy should render form controls").toBeGreaterThanOrEqual(10);
    for (let index = 0; index < fieldCount; index++) {
      await expectElementMinSize(fieldControls.nth(index), {
        label: `hybrid policy field control ${index}`,
        minHeight: UI_LIMITS.inputMinHeight
      });
    }

    const checkboxGrid = page.getByTestId("hybrid-policy-checkbox-grid");
    await expect(checkboxGrid).toBeVisible();
    const display = await checkboxGrid.evaluate((element) => getComputedStyle(element).display);
    expect(display, "hybrid policy checkbox container should use grid layout").toBe("grid");

    const checkboxFields = checkboxGrid.locator(".checkbox-field");
    await expect(checkboxFields).toHaveCount(5);
    const checkboxHeights = await checkboxFields.evaluateAll((fields) => fields.map((field) => field.getBoundingClientRect().height));
    const firstHeight = checkboxHeights[0];
    for (const [index, height] of checkboxHeights.entries()) {
      expect(Math.abs(height - firstHeight), `checkbox field ${index} should align to the same height`).toBeLessThanOrEqual(1);
    }

    await expect(section.getByText(/TTL máximo do pacote|Max package TTL/i)).toBeVisible();
    await expect(section.getByText(/Permitir delimiter S3|Allow S3 delimiter/i)).toBeVisible();

    const s3Advanced = page.getByTestId("s3-advanced-policy-section");
    await expect(s3Advanced).toBeVisible();
    await expect(s3Advanced.getByText(/S3 avançado|Advanced S3/i)).toBeVisible();
    await expect(s3Advanced.getByLabel(/Criptografia padrão|Default encryption/i)).toBeVisible();
    await expect(s3Advanced.getByLabel(/Object Lock/i)).toBeVisible();
    await expect(s3Advanced.getByLabel(/Lifecycle rules JSON|Regras lifecycle JSON/i)).toBeVisible();
    const jsonEditors = s3Advanced.locator("textarea");
    await expect(jsonEditors).toHaveCount(3);
    for (let index = 0; index < 3; index++) {
      await expectElementMinSize(jsonEditors.nth(index), {
        label: `S3 JSON editor ${index}`,
        minHeight: 120
      });
    }
    await expectNoHorizontalOverflow(page);
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

  test("buckets created outside the panel should appear automatically", async ({ page }) => {
    let requests = 0;
    await page.route("**/api/admin/buckets?*", async (route) => {
      requests += 1;
      const items = requests > 1
        ? [
            { name: "assets", objectCount: 2, totalBytes: 3072, createdAt: "2026-06-30T12:00:00.000Z" },
            { name: "external-bucket", objectCount: 0, totalBytes: 0, createdAt: "2026-06-30T12:00:00.000Z" }
          ]
        : [{ name: "assets", objectCount: 2, totalBytes: 3072, createdAt: "2026-06-30T12:00:00.000Z" }];
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ items, page: 1, pageSize: 20, total: items.length, totalPages: 1 })
      });
    });

    await openBucketsPage(page);
    await expect(page.getByText("external-bucket", { exact: true })).toBeVisible({ timeout: 7000 });
  });
});

async function openBucketsPage(page: import("@playwright/test").Page) {
  await page.goto("/dashboard");
  await page.getByTestId("app-sidebar").getByRole("link", { name: /buckets/i }).click();
  await expect(page).toHaveURL(/\/buckets$/);
}
