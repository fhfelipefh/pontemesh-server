import { expect, test } from "@playwright/test";
import { installAdminApiFixtures } from "./helpers/adminApiFixtures";
import { expectElementMaxSize, expectNoHorizontalOverflow } from "./helpers/layoutAssertions";

test.describe("Settings MCP layout quality", () => {
  test.beforeEach(async ({ page }) => {
    await installAdminApiFixtures(page);
  });

  test("token scope checkboxes should stay compact and aligned", async ({ page }) => {
    await page.goto("/settings");
    await page.getByRole("button", { name: /tokens de acesso/i }).click();

    const scopeGroup = page.getByTestId("mcp-token-scope-group");
    await expect(scopeGroup).toBeVisible();

    for (const scope of ["read", "write", "admin"]) {
      const checkbox = page.getByTestId(`mcp-token-scope-${scope}`);
      await expect(checkbox).toBeVisible();
      await expectElementMaxSize(checkbox, {
        label: `MCP ${scope} scope checkbox`,
        maxWidth: 20,
        maxHeight: 20
      });
      await expect(scopeGroup.getByText(scope, { exact: true })).toBeVisible();
    }

    const scopeOverflow = await scopeGroup.evaluate((element) => element.scrollWidth - element.clientWidth);
    expect(scopeOverflow, "MCP token scope group should not overflow horizontally").toBeLessThanOrEqual(1);

    const createTokenButton = page.getByRole("button", { name: /create token|criar token/i });
    await expect(createTokenButton).toBeVisible();
    await expectElementMaxSize(createTokenButton, {
      label: "MCP create token button",
      maxWidth: 280
    });

    await expect(page.getByText(/Write and admin scopes change server data|Escopos write e admin alteram dados do servidor/i)).toBeHidden();
    await page.getByTestId("mcp-token-scope-write").check();
    await expect(page.getByText(/Write and admin scopes change server data|Escopos write e admin alteram dados do servidor/i)).toBeVisible();

    await expectNoHorizontalOverflow(page);
  });
});
