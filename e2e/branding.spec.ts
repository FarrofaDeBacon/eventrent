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

    // 6. Alterna toggle de prévia Claro / Escuro e valida componentes do preview (botão, badge, card, sidebar, tabela)
    await expect(
      page.locator(".sticky").getByText("Barra Lateral (Navegação)"),
    ).toBeVisible();
    await expect(
      page.locator(".sticky").getByText("Tabela de Itens"),
    ).toBeVisible();
    await expect(page.locator(".sticky table")).toBeVisible();

    const darkPreviewBtn = page.getByRole("button", {
      name: "Visualizar Escuro",
    });
    await darkPreviewBtn.click();
    await expect(page.locator(".sticky .dark")).toBeVisible();
    await expect(page.locator(".sticky .dark table")).toBeVisible();

    const lightPreviewBtn = page.getByRole("button", {
      name: "Visualizar Claro",
    });
    await lightPreviewBtn.click();
    await expect(page.locator(".sticky .dark")).not.toBeVisible();

    // 7. Áreas de upload por arrastar e soltar (drag & drop) e conferência de fundos
    await expect(
      page.locator('[aria-label="Área de upload do logo claro"]'),
    ).toBeVisible();
    await expect(
      page.locator('[aria-label="Área de upload do logo escuro"]'),
    ).toBeVisible();
    await expect(
      page.locator('[aria-label="Área de upload do ícone"]'),
    ).toBeVisible();

    // Upload de logo claro (PNG) para validação da conferência de fundos claro e escuro
    await page
      .locator('input[type="file"]')
      .first()
      .setInputFiles({
        name: "logo.png",
        mimeType: "image/png",
        buffer: Buffer.from(
          "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
          "base64",
        ),
      });
    await expect(
      page.getByText("Conferência Visual: Fundo Claro vs. Fundo Escuro"),
    ).toBeVisible();
    await expect(
      page.locator('img[alt="Logo sobre fundo claro"]'),
    ).toBeVisible();
    await expect(
      page.locator('img[alt="Logo claro sobre fundo escuro"]'),
    ).toBeVisible();

    // 8. Validação ao vivo de campo HEX inválido
    const primaryInput = page.getByLabel("Cor Primária").nth(1); // text input
    await primaryInput.fill("#xyz");
    await expect(
      page.getByText("Formato inválido. Use #RRGGBB."),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Salvar Alterações" }),
    ).toBeDisabled();

    // Restaura cor válida
    await primaryInput.fill("#059669");
    await expect(
      page.getByText("Formato inválido. Use #RRGGBB."),
    ).not.toBeVisible();
    await expect(
      page.getByRole("button", { name: "Salvar Alterações" }),
    ).toBeEnabled();

    // 8. Correção automática de contraste quando < 4.5:1
    await primaryInput.fill("#888888");
    const autoFixBtn = page.getByRole("button", {
      name: "Corrigir Contraste Automaticamente",
    });
    await expect(autoFixBtn).toBeVisible();
    await autoFixBtn.click();
    await expect(autoFixBtn).not.toBeVisible();
    await expect(page.getByText(/ajustada automaticamente/i)).toBeVisible();

    // 9. Salva as alterações
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
