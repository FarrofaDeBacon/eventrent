import { test, expect } from "@playwright/test";

test.describe("Teste de Fumaça — EventRent (Fase 0)", () => {
  test("deve carregar a página inicial e exibir o App Shell", async ({
    page,
  }) => {
    await page.goto("/");

    // Verifica que o shell carregou o nome do app
    await expect(page.locator("h1")).toContainText("EventRent");

    // Verifica presença do card de boas-vindas
    await expect(page.getByText("Fase 0 Concluída")).toBeVisible();
    await expect(page.getByText("Performance Extrema")).toBeVisible();
  });

  test("deve navegar até a tela de Aparência e exibir formulário de branding", async ({
    page,
  }) => {
    await page.goto("/configuracoes/aparencia");

    // Verifica título da página
    await expect(
      page.getByRole("heading", { name: "Aparência e Identidade Visual" }),
    ).toBeVisible();

    // Verifica inputs de branding
    await expect(page.getByLabel("Nome da Empresa")).toBeVisible();
    await expect(page.getByLabel("Cor Primária")).toBeVisible();
    await expect(page.getByLabel("Cor Secundária")).toBeVisible();
    await expect(page.getByLabel("Cor de Destaque")).toBeVisible();

    // Verifica botão de salvar
    await expect(
      page.getByRole("button", { name: "Salvar Alterações" }),
    ).toBeVisible();
  });
});
