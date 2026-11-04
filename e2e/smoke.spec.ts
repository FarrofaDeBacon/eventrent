import { test, expect } from "@playwright/test";

test.describe("Teste de Fumaça — EventRent (Fase 0)", () => {
  test("deve carregar a página inicial e exibir o App Shell", async ({
    page,
  }) => {
    await page.goto("/");

    await expect(page.locator("h1")).toContainText("EventRent");
    await expect(page.getByText("Fase 0 Concluída")).toBeVisible();
    await expect(page.getByText("Performance Extrema")).toBeVisible();
  });

  test("deve navegar até a tela de Aparência, alterar branding e persistir", async ({
    page,
  }) => {
    await page.goto("/configuracoes/aparencia");

    await expect(
      page.getByRole("heading", { name: "Aparência e Identidade Visual" }),
    ).toBeVisible();

    const nameInput = page.getByLabel("Nome da Empresa");
    await expect(nameInput).toBeVisible();
    await expect(page.locator("#primaryColor")).toBeVisible();
    await expect(page.locator("#secondaryColor")).toBeVisible();
    await expect(page.locator("#accentColor")).toBeVisible();

    await nameInput.fill("Luz & Som Master");
    await page.getByRole("button", { name: "Salvar Alterações" }).click();

    await expect(
      page.getByText("Identidade visual salva e aplicada com sucesso!"),
    ).toBeVisible();
    await expect(page.locator("h1")).toContainText("Luz & Som Master");
  });
});
