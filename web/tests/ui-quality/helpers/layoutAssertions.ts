import { expect, Locator, Page } from "@playwright/test";

export const UI_LIMITS = {
  inputMinHeight: 36,
  inputMinWidth: 180,
  searchInputMinWidth: 280,
  buttonMinHeight: 32,
  buttonMinWidth: 32,
  buttonMaxWidth: 420,
  primaryButtonMaxWidthDesktop: 280
};

export async function expectElementMinSize(
  locator: Locator,
  options: { minWidth?: number; minHeight?: number; label: string }
) {
  const box = await locator.boundingBox();
  expect(box, `${options.label} should be visible and measurable`).not.toBeNull();

  if (!box) return;

  if (options.minWidth !== undefined) {
    expect(
      box.width,
      `${options.label} width should be at least ${options.minWidth}px`
    ).toBeGreaterThanOrEqual(options.minWidth);
  }

  if (options.minHeight !== undefined) {
    expect(
      box.height,
      `${options.label} height should be at least ${options.minHeight}px`
    ).toBeGreaterThanOrEqual(options.minHeight);
  }
}

export async function expectElementMaxSize(
  locator: Locator,
  options: { maxWidth?: number; maxHeight?: number; label: string }
) {
  const box = await locator.boundingBox();
  expect(box, `${options.label} should be visible and measurable`).not.toBeNull();

  if (!box) return;

  if (options.maxWidth !== undefined) {
    expect(
      box.width,
      `${options.label} width should be at most ${options.maxWidth}px`
    ).toBeLessThanOrEqual(options.maxWidth);
  }

  if (options.maxHeight !== undefined) {
    expect(
      box.height,
      `${options.label} height should be at most ${options.maxHeight}px`
    ).toBeLessThanOrEqual(options.maxHeight);
  }
}

export async function expectNoHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(() => {
    return document.documentElement.scrollWidth - document.documentElement.clientWidth;
  });

  expect(overflow, "page should not have horizontal overflow").toBeLessThanOrEqual(1);
}

export async function expectBodyDoesNotScroll(page: Page) {
  const extraScroll = await page.evaluate(() => {
    return document.documentElement.scrollHeight - document.documentElement.clientHeight;
  });

  expect(extraScroll, "page should not require vertical body scroll").toBeLessThanOrEqual(1);
}

export async function expectNoTinyTextInputs(page: Page) {
  const inputs = page.locator("input:not([type=hidden]):not([type=checkbox]):not([type=radio]), textarea");
  const count = await inputs.count();

  for (let i = 0; i < count; i++) {
    const input = inputs.nth(i);
    const box = await input.boundingBox();

    if (!box || box.width === 0 || box.height === 0) continue;

    expect(box.height, `input ${i} height should be usable`).toBeGreaterThanOrEqual(UI_LIMITS.inputMinHeight);
    expect(box.width, `input ${i} width should be readable`).toBeGreaterThanOrEqual(UI_LIMITS.inputMinWidth);
  }
}

export async function expectButtonsHaveUsableSize(page: Page) {
  const buttons = page.locator("button");
  const count = await buttons.count();

  for (let i = 0; i < count; i++) {
    const button = buttons.nth(i);
    const box = await button.boundingBox();

    if (!box || box.width === 0 || box.height === 0) continue;

    expect(box.height, `button ${i} height should be usable`).toBeGreaterThanOrEqual(UI_LIMITS.buttonMinHeight);
    expect(box.width, `button ${i} width should not collapse`).toBeGreaterThanOrEqual(UI_LIMITS.buttonMinWidth);
    expect(box.width, `button ${i} width should not become a giant bar accidentally`).toBeLessThanOrEqual(UI_LIMITS.buttonMaxWidth);
  }
}

export async function expectButtonHasIcon(locator: Locator, label: string) {
  await expect(locator, `${label} should be visible`).toBeVisible();
  await expect(locator.locator("svg"), `${label} should include a standard action icon`).toHaveCount(1);
}
