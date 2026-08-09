import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.route("**/api/setup/status", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        setupRequired: true,
        serverVersion: "0.2.2",
        internalWebPort: 8080,
        internalS3Port: 9000,
        publicWebUrl: "https://134.65.234.41",
        publicS3Url: "https://134.65.234.41:9443"
      })
    });
  });
});

test("shows internal listeners separately from public Origin endpoints", async ({ page }) => {
  await page.goto("/setup");
  await page.locator("#token").fill("setup-token");
  await page.route("**/api/setup/unlock", async (route) => {
    await route.fulfill({ status: 200, contentType: "application/json", body: "{}" });
  });
  await page.getByRole("button", { name: /continue|continuar/i }).click();

  await expect(page).toHaveURL(/\/setup\/configure$/);
  await expect(page.getByRole("heading", { name: /configure ponte mesh|configurar ponte mesh/i })).toBeVisible();
  await expect(page.locator("#httpPort")).toHaveCount(0);
  await expect(page.getByText(/internal panel port|porta interna do painel/i)).toBeVisible();
  await expect(page.getByText(/internal s3 port|porta interna s3/i)).toBeVisible();
  await expect(page.locator("#publicWebUrl")).toHaveValue("https://134.65.234.41");
  await expect(page.locator("#publicS3Url")).toHaveValue("https://134.65.234.41:9443");

  const cardBox = await page.locator(".setup-card").boundingBox();
  const helpBox = await page.locator(".help-link").boundingBox();
  expect(cardBox).not.toBeNull();
  expect(helpBox).not.toBeNull();
  expect(helpBox!.y - (cardBox!.y + cardBox!.height)).toBeGreaterThanOrEqual(18);

  await page.locator("#role").selectOption("replica-edge");
  await expect(page.locator("#publicWebUrl")).toHaveCount(0);
  await expect(page.locator("#originBaseUrl")).toBeVisible();
});

test("keeps only the fixed version centered at the bottom on desktop and mobile", async ({ page }) => {
  for (const viewport of [{ width: 1440, height: 900 }, { width: 390, height: 844 }]) {
    await page.setViewportSize(viewport);
    await page.goto("/setup");

    const version = page.locator(".setup-page__version");
    await expect(version).toHaveText("v0.2.2");
    await expect(page.getByText("Ponte Mesh Server v0.2.2", { exact: true })).toHaveCount(0);

    const position = await version.evaluate((element) => getComputedStyle(element).position);
    expect(position).toBe("fixed");
    const box = await version.boundingBox();
    expect(box).not.toBeNull();
    expect(Math.abs((box!.x + box!.width / 2) - viewport.width / 2)).toBeLessThanOrEqual(1);
    expect(viewport.height - (box!.y + box!.height)).toBeGreaterThanOrEqual(10);
    expect(viewport.height - (box!.y + box!.height)).toBeLessThanOrEqual(14);
  }
});
