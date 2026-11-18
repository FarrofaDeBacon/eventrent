import { test, expect } from "@playwright/test";

test.describe("E2E — Locais de Evento, Espaços e Operação (Fase 1D)", () => {
  test("deve configurar tempos operacionais, cadastrar Riocentro com 3 pavilhões, editar e validar RBAC", async ({
    page,
  }) => {
    // 1. Setup inicial como Proprietário
    await page.goto("/");
    await expect(page).toHaveURL(/\/setup$/);

    await page.getByLabel("Nome da sua Empresa").fill("Eventos & Palco Brasil");
    await page.getByLabel("Seu Nome Completo").fill("Ana Proprietária");
    await page.getByLabel("E-mail de Acesso").fill("ana@eventospalco.com.br");
    await page
      .getByLabel("Senha de Acesso (mínimo 8 caracteres)")
      .fill("AnaSegura#2026");
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

    // 2. Configurações > Operação (/settings/operation)
    await page.getByRole("link", { name: "Operação", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/operation$/);

    await expect(
      page.getByLabel("Montagem padrão (min)", { exact: true }),
    ).toHaveValue("30");
    await page.getByLabel("Montagem padrão (min)", { exact: true }).fill("45");
    await page.getByLabel("Desmontagem padrão (min)").fill("25");
    await page.getByLabel("Buffer de giro (min)").fill("90");
    await page.getByRole("button", { name: "Salvar configurações" }).click();
    await expect(
      page.getByText("Configurações operacionais salvas com sucesso."),
    ).toBeVisible();

    // 3. Navega para Locais (/venues) e cadastra Riocentro com 3 pavilhões
    await page.getByRole("link", { name: "Locais", exact: true }).click();
    await expect(page).toHaveURL(/\/venues$/);
    await page.getByRole("link", { name: "Novo local" }).click();
    await expect(page).toHaveURL(/\/venues\/new$/);

    await page.getByLabel("Nome do local *").fill("Riocentro");
    await page.getByLabel("Cidade").fill("Rio de Janeiro");
    await page.getByLabel("UF").fill("RJ");

    await page.getByRole("tab", { name: /Espaços/i }).click();
    for (const [idx, pav] of [
      "Pavilhão 1",
      "Pavilhão 2",
      "Pavilhão 3",
    ].entries()) {
      await page.getByRole("button", { name: "Adicionar espaço" }).click();
      await page.getByLabel("Nome do espaço *").nth(idx).fill(pav);
      await page
        .getByLabel("Desloc. interno (min)")
        .nth(idx)
        .fill(String((idx + 1) * 10));
    }

    await page.getByRole("tab", { name: /Contatos/i }).click();
    await page.getByRole("button", { name: "Adicionar contato" }).click();
    await page.getByLabel("Nome *").fill("Carlos Doca");
    await page.getByLabel("E-mail").fill("doca@riocentro.com.br");

    await page.getByRole("button", { name: "Salvar local" }).click();
    await expect(page).toHaveURL(/\/venues\/.+$/);
    await expect(
      page.getByRole("heading", { name: "Riocentro" }),
    ).toBeVisible();

    // 4. Confirma na lista e busca por pavilhão
    await page.getByRole("link", { name: "Locais", exact: true }).click();
    await expect(page.getByText("3 espaços")).toBeVisible();
    await page
      .getByPlaceholder("Buscar por nome, cidade, espaço ou contato...")
      .fill("Pavilhão 2");
    await expect(page.getByText("Riocentro", { exact: true })).toBeVisible();

    // 5. Cria usuário Técnico e valida modo somente leitura em /venues/[id]
    await page.getByRole("link", { name: "Usuários", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/users$/);
    await page.getByTestId("new-user-btn").click();
    await page.getByLabel("Nome Completo").fill("Lucas Técnico");
    await page.getByLabel("E-mail").fill("lucas@eventospalco.com.br");
    await page
      .getByLabel("Senha Inicial (mínimo 8 caracteres)")
      .fill("Tecnico#2026");
    await page.getByLabel("Perfil de Acesso").selectOption("role-technician");
    await page.getByRole("button", { name: "Salvar" }).click();
    await expect(
      page.getByText("Usuário cadastrado com sucesso!").first(),
    ).toBeVisible();

    await page.getByRole("button", { name: "Sair" }).click();
    await expect(page).toHaveURL(/\/login$/);

    await page.getByLabel("E-mail").fill("lucas@eventospalco.com.br");
    await page.getByLabel("Senha").fill("Tecnico#2026");
    await page.getByRole("button", { name: "Entrar" }).click();
    await expect(page).toHaveURL(/\/$/);

    await page.getByRole("link", { name: "Locais", exact: true }).click();
    await expect(page).toHaveURL(/\/venues$/);
    await expect(
      page.getByRole("link", { name: "Novo local" }),
    ).not.toBeVisible();

    await page.getByText("Riocentro", { exact: true }).click();
    await expect(page).toHaveURL(/\/venues\/.+$/);
    await expect(
      page.getByRole("button", { name: "Salvar local" }),
    ).not.toBeVisible();
  });
});
