import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";
import {
  expectButtonsHaveUsableSize,
  expectNoHorizontalOverflow,
  expectNoTinyTextInputs
} from "./helpers/layoutAssertions";

test.describe("Objects page", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  test("sidebar navigation, bucket selection, search, upload modal, and delete modal should work", async ({ page }) => {
    const pageErrors: string[] = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));

    await page.goto("/dashboard");
    await page.getByTestId("app-sidebar").getByRole("link", { name: /objects|objetos/i }).click();
    await expect(page).toHaveURL(/\/objects(\?bucket=assets)?$/);

    await expect(page.getByRole("heading", { name: /objects|objetos/i })).toBeVisible();
    await expect(page.getByTestId("objects-bucket-select")).toHaveValue("assets");
    await expect(page.getByTestId("object-list")).toBeVisible();
    await expect(page.getByText("images/logo.png")).toBeVisible();

    await page.getByTestId("objects-bucket-select").selectOption("documents");
    await expect(page).toHaveURL(/\/objects\?bucket=documents$/);

    await page.getByTestId("object-search-input").fill("readme");
    await page.getByTestId("object-search-input").press("Enter");
    await expect(page.getByText("docs/readme.txt")).toBeVisible();

    await expect(page.getByTestId("object-key-input")).toHaveCount(0);
    await expect(page.getByText(/no file chosen|nenhum arquivo escolhido/i)).toHaveCount(0);

    await page.getByTestId("open-upload-object-button").click();
    await expect(page.getByTestId("upload-object-dialog")).toBeVisible();
    await expect(page.getByTestId("object-key-input")).toBeVisible();
    await expect(page.getByText(/choose file|escolher arquivo/i)).toBeVisible();

    await page.getByTestId("object-file-input").setInputFiles({
      name: "objects-page-upload.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("pontemesh objects page upload")
    });
    await expect(page.getByTestId("upload-object-button")).toBeEnabled();
    await page.getByTestId("upload-object-button").click();
    await expect(page.getByTestId("upload-object-dialog")).toBeHidden();
    await expect(page.getByRole("complementary", { name: /uploads/i })).toBeVisible();
    await page.getByRole("button", { name: /close upload|fechar upload/i }).click();
    await expect(page.getByRole("complementary", { name: /uploads/i })).toBeHidden();

    await page.getByTestId("object-row").first().getByRole("button", { name: /delete|excluir/i }).click();
    await expect(page.getByTestId("confirm-dialog")).toBeVisible();
    await page.getByTestId("confirm-dialog").getByRole("button", { name: /confirm|confirmar/i }).click();
    await expect(page.getByTestId("confirm-dialog")).toBeHidden();

    expect(pageErrors).toEqual([]);
    await expectNoHorizontalOverflow(page);
  });

  test("empty bucket and empty object states should render", async ({ page }) => {
    await page.unroute("**/api/**");
    await installAdminApiFixtures(page, { buckets: [] });
    await page.goto("/objects");
    await expect(page.getByText(/no buckets are available|nenhum bucket disponível/i)).toBeVisible();

    await page.unroute("**/api/**");
    await installAdminApiFixtures(page, {
      objectsByBucket: {
        assets: [],
        documents: []
      }
    });
    await page.goto("/objects?bucket=assets");
    await expect(page.getByText(/does not have objects yet|ainda não possui objetos/i)).toBeVisible();
    await page.getByTestId("open-upload-object-button").click();
    await expect(page.getByTestId("upload-object-dialog")).toBeVisible();
  });

  test("sidebar collapsed and expanded states should keep the Objects item usable", async ({ page }) => {
    await page.goto("/objects");
    await expect(page.getByTestId("app-sidebar").getByRole("link", { name: /objects|objetos/i })).toBeVisible();
    await page.getByTestId("sidebar-toggle").click();
    await expect(page.getByTestId("app-sidebar").getByRole("link", { name: /objects|objetos/i })).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await expectNoTinyTextInputs(page);
    await expectButtonsHaveUsableSize(page);
  });

  test("main visible labels should switch between EN and PT-BR", async ({ page }) => {
    await page.goto("/objects");
    await expect(page.getByRole("heading", { name: "Objects" })).toBeVisible();

    await page.getByLabel(/language|idioma/i).click();
    await page.getByRole("menuitemradio", { name: /pt-br|português/i }).click();
    await expect(page.getByRole("heading", { name: "Objetos" })).toBeVisible();
    await expect(page.getByText("Enviar objeto")).toBeVisible();
  });

  test("main object controls should stay proportional and aligned", async ({ page }) => {
    await page.goto("/objects");
    await expect(page.getByTestId("object-search-input")).toBeVisible();

    const searchBox = await page.getByTestId("object-search-input").boundingBox();
    const refreshButton = await page.getByRole("button", { name: /refresh|atualizar/i }).boundingBox();
    const uploadButton = await page.getByTestId("open-upload-object-button").boundingBox();
    const tableCard = await page.getByTestId("objects-table-card").boundingBox();
    const pagination = await page.locator(".objects-table-card .buckets-pagination").boundingBox();

    expect(searchBox, "search input should be measurable").not.toBeNull();
    expect(refreshButton, "refresh button should be measurable").not.toBeNull();
    expect(uploadButton, "upload button should be measurable").not.toBeNull();
    expect(tableCard, "table card should be measurable").not.toBeNull();
    expect(pagination, "pagination should be measurable").not.toBeNull();

    expect(searchBox!.width).toBeGreaterThanOrEqual(280);
    expect(refreshButton!.width).toBeLessThanOrEqual(180);
    expect(uploadButton!.width).toBeLessThanOrEqual(220);
    expect(pagination!.y + pagination!.height).toBeLessThanOrEqual(tableCard!.y + tableCard!.height + 1);

    for (const button of await page.locator(".objects-table-actions .table-action-button").all()) {
      const box = await button.boundingBox();
      expect(box, "table action button should be measurable").not.toBeNull();
      expect(box!.width).toBeLessThanOrEqual(140);
      expect(box!.height).toBeGreaterThanOrEqual(32);
      expect(box!.height).toBeLessThanOrEqual(44);
    }

    await expectNoHorizontalOverflow(page);
  });
});
