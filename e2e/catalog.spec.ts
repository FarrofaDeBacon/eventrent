import { test, expect } from "@playwright/test";

test.describe("E2E — Catálogo de Equipamentos, Categorias e Atributos (Fase 2A)", () => {
  test("deve carregar categorias sugeridas, criar marca inline, cadastrar modelo com campo herdado/próprio e validar modo somente leitura para Técnico", async ({
    page,
  }) => {
    // 1. Setup inicial como Proprietário
    await page.goto("/");
    await expect(page).toHaveURL(/\/setup$/);

    await page.getByLabel("Nome da sua Empresa").fill("Som & Luz Eventos");
    await page.getByLabel("Seu Nome Completo").fill("Carlos Proprietário");
    await page.getByLabel("E-mail de Acesso").fill("carlos@somluz.com.br");
    await page
      .getByLabel("Senha de Acesso (mínimo 8 caracteres)")
      .fill("CarlosSegura#2026");
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

    // 2. Acessar Catálogo -> Categorias e Atributos
    await page.getByRole("link", { name: "Equipamentos", exact: true }).click();
    await expect(page).toHaveURL(/\/catalog$/);

    await page.getByRole("link", { name: "Categorias e Atributos" }).click();
    await expect(page).toHaveURL(/\/catalog\/categories$/);

    await page
      .getByRole("button", { name: "Carregar categorias sugeridas" })
      .click();
    await expect(page.getByText(/Sugestões carregadas:/)).toBeVisible();

    // Seleciona "Mesas de som" (subcategoria de Áudio)
    await page.getByRole("button", { name: /↳ Mesas de som/ }).click();
    await expect(page.getByText("herdado de Áudio")).toBeVisible();
    await expect(page.getByText("canais", { exact: true })).toBeVisible();

    // 3. Cadastrar novo modelo com marca inline e campo técnico "Canais"
    await page.getByLabel("Voltar para catálogo").click();
    await expect(page).toHaveURL(/\/catalog$/);

    await page.getByRole("link", { name: "Novo modelo" }).click();
    await expect(page).toHaveURL(/\/catalog\/new$/);

    await page.getByLabel("Nome do modelo *").fill("X32 Compact");

    // Seleciona categoria "Mesas de som"
    const catSelect = page.getByLabel("Categoria *");
    const mesasOption = await catSelect
      .locator("option", { hasText: "Mesas de som" })
      .getAttribute("value");
    expect(mesasOption).toBeTruthy();
    await catSelect.selectOption(mesasOption!);

    // Cria marca inline "Behringer"
    await page.getByRole("button", { name: "Nova marca" }).click();
    await page.getByLabel("Nome da nova marca").fill("Behringer");
    await page.getByRole("button", { name: "Adicionar", exact: true }).click();
    await expect(page.getByText("Marca 'Behringer' criada.")).toBeVisible();

    await page.getByLabel("SKU interno").fill("AUD-X32C");
    await page.getByLabel("Diária padrão (R$)").fill("350.00");
    await page.getByLabel(/Canais \(ch\)/).fill("32");

    await page.getByRole("button", { name: "Salvar modelo" }).click();
    await expect(page).toHaveURL(/\/catalog\/model-/);
    await expect(
      page.getByRole("heading", { name: "X32 Compact" }),
    ).toBeVisible();

    // 4. Buscar na listagem do catálogo
    await page.getByLabel("Voltar para catálogo").click();
    await expect(page).toHaveURL(/\/catalog$/);
    await page.getByLabel("Buscar modelos no catálogo").fill("X32");
    await expect(page.getByText("X32 Compact")).toBeVisible();
    await expect(page.getByText(/• Behringer/)).toBeVisible();

    // 5. Criar usuário Técnico e verificar modo somente leitura
    await page.getByRole("link", { name: "Usuários", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/users$/);
    await page.getByTestId("new-user-btn").click();
    await page.getByLabel("Nome Completo").fill("Lucas Técnico");
    await page.getByLabel("E-mail").fill("lucas@somluz.com.br");
    await page
      .getByLabel("Senha Inicial (mínimo 8 caracteres)")
      .fill("Tecnico#2026");
    await page.getByLabel("Perfil de Acesso").selectOption("role-technician");
    await page.getByRole("button", { name: "Salvar" }).click();
    await expect(page.getByText("lucas@somluz.com.br")).toBeVisible();

    // Sair e entrar como Técnico
    await page.getByRole("button", { name: "Sair" }).click();
    await expect(page).toHaveURL(/\/login$/);
    await page.getByLabel("E-mail").fill("lucas@somluz.com.br");
    await page.getByLabel("Senha").fill("Tecnico#2026");
    await page.getByRole("button", { name: "Entrar" }).click();
    await expect(page).toHaveURL(/\/$/);

    await page.getByRole("link", { name: "Equipamentos", exact: true }).click();
    await expect(page).toHaveURL(/\/catalog$/);
    await expect(page.getByRole("link", { name: "Novo modelo" })).toHaveCount(
      0,
    );

    await page.getByText("X32 Compact").click();
    await expect(page.getByText(/Modo somente leitura/)).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Salvar modelo" }),
    ).toHaveCount(0);
  });
});
