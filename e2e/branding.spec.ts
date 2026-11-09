import { test, expect } from "@playwright/test";

test.describe("E2E — Identidade Visual e White Label (Fase 1B.1)", () => {
  test("deve configurar branding, selecionar tema predefinido, validar preview em tempo real e persistir alterações", async ({
    page,
  }) => {
    // 1. Setup inicial
    await page.goto("/");
    await expect(page).toHaveURL(/\/setup$/);

    await page.getByLabel("Nome da sua Empresa").fill("Som & Iluminação Prime");
    await page.getByLabel("Seu Nome Completo").fill("Roberto Dono");
    await page.getByLabel("E-mail de Acesso").fill("roberto@prime.com.br");
    await page
      .getByLabel("Senha de Acesso (mínimo 8 caracteres)")
      .fill("Roberto#2026");

    await page
      .getByRole("button", { name: "Concluir Configuração Inicial" })
      .click();

    await page
      .getByLabel(
        "Anotei e guardei minha Chave de Recuperação em um local seguro",
      )
      .check();
    await page.getByRole("button", { name: "Entrar no Sistema" }).click();

    await expect(page).toHaveURL(/\/$/);
    await expect(page.locator("h1")).toContainText("Som & Iluminação Prime");

    // 2. Navega para Aparência (/settings/appearance)
    await page.getByRole("link", { name: "Aparência", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/appearance$/);

    await expect(
      page.getByRole("heading", { name: "Aparência e Identidade Visual" }),
    ).toBeVisible();

    // 3. Testa seleção de tema predefinido "Esmeralda Tech"
    const emeraldPreset = page.getByRole("button", { name: /Esmeralda Tech/i });
    await expect(emeraldPreset).toBeVisible();
    await emeraldPreset.click();

    // 4. Altera o nome da empresa
    const companyInput = page.getByLabel("Nome da Empresa");
    await companyInput.fill("EventRent White Label Pro");

    // 5. Verifica preview em tempo real
    await expect(page.locator(".sticky")).toContainText(
      "EventRent White Label Pro",
    );

    // 6. Alterna toggle de prévia Claro / Escuro
    const darkPreviewBtn = page.getByRole("button", {
      name: "Visualizar Escuro",
    });
    await darkPreviewBtn.click();
    await expect(page.locator(".sticky .dark")).toBeVisible();

    const lightPreviewBtn = page.getByRole("button", {
      name: "Visualizar Claro",
    });
    await lightPreviewBtn.click();
    await expect(page.locator(".sticky .dark")).not.toBeVisible();

    // 7. Salva as alterações
    await page.getByRole("button", { name: "Salvar Alterações" }).click();
    await expect(
      page.getByText("Identidade visual salva e aplicada com sucesso!"),
    ).toBeVisible();

    // 8. Recarrega a página e valida persistência e document.title
    await page.reload();
    await expect(companyInput).toHaveValue("EventRent White Label Pro");
    await expect(page).toHaveTitle(/EventRent White Label Pro/);

    // 9. Faz logout e valida que a tela de login exibe a marca customizada
    await page.goto("/");
    await page.getByRole("button", { name: "Sair" }).click();
    await expect(page).toHaveURL(/\/login$/);
    await expect(page.locator("h1")).toContainText("EventRent White Label Pro");
  });
});

