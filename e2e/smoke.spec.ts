import { test, expect } from "@playwright/test";

test.describe("Fluxo E2E — EventRent (Fase 1A)", () => {
  test("deve executar primeira execução (modo local + proprietário + chave de recuperação), fazer login, criar usuário e navegar pelas telas", async ({
    page,
  }) => {
    // 1. Primeira execução: redireciona automaticamente de / para /setup
    await page.goto("/");
    await expect(page).toHaveURL(/\/setup$/);

    await expect(
      page.getByRole("heading", { name: "Bem-vindo ao EventRent" }),
    ).toBeVisible();

    // Verifica os dois modos de operação: Local ativo e Nuvem visível com "Em breve"
    const localCard = page.getByTestId("mode-local-card");
    const cloudCard = page.getByTestId("mode-cloud-card");
    await expect(localCard).toContainText("Neste computador");
    await expect(cloudCard).toContainText("Em mais de um computador (nuvem)");
    await expect(cloudCard).toContainText("Em breve");

    // 2. Preenche dados da empresa e conta do Proprietário
    await page.getByLabel("Nome da sua Empresa").fill("Palco & Luz Eventos");
    await page.getByLabel("Seu Nome Completo").fill("Ana Paula Proprietária");
    await page.getByLabel("E-mail de Acesso").fill("ana@palcoeluz.com.br");
    await page
      .getByLabel("Senha de Acesso (mínimo 8 caracteres)")
      .fill("AnaSegura#2026");

    await page
      .getByRole("button", { name: "Concluir Configuração Inicial" })
      .click();

    // 3. Confirma exibição única da Chave de Recuperação
    await expect(
      page.getByRole("heading", { name: "Guarde sua Chave de Recuperação" }),
    ).toBeVisible();
    const recoveryKeyEl = page.getByTestId("recovery-key-value");
    await expect(recoveryKeyEl).toBeVisible();
    const recoveryKeyText = await recoveryKeyEl.innerText();
    expect(recoveryKeyText.trim()).toMatch(
      /^ER-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/,
    );

    await page
      .getByLabel(
        "Anotei e guardei minha Chave de Recuperação em um local seguro",
      )
      .check();
    await page.getByRole("button", { name: "Entrar no Sistema" }).click();

    // 4. Painel principal autenticado
    await expect(page).toHaveURL(/\/$/);
    await expect(page.locator("h1")).toContainText("Palco & Luz Eventos");
    await expect(page.getByTestId("header-user-info")).toContainText(
      "Ana Paula Proprietária",
    );

    // 5. Faz logout e testa tela de Login (/login)
    await page.getByRole("button", { name: "Sair" }).click();
    await expect(page).toHaveURL(/\/login$/);

    await page.getByLabel("E-mail").fill("ana@palcoeluz.com.br");
    await page.getByLabel("Senha").fill("AnaSegura#2026");
    await page.getByRole("button", { name: "Entrar" }).click();

    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByTestId("header-user-info")).toContainText(
      "Ana Paula Proprietária",
    );

    // 6. Navega para Usuários (/configuracoes/usuarios) e cria um novo usuário
    await page.getByRole("link", { name: "Usuários", exact: true }).click();
    await expect(page).toHaveURL(/\/configuracoes\/usuarios$/);

    await page.getByTestId("new-user-btn").click();
    await page.getByLabel("Nome Completo").fill("Carlos Comercial");
    await page.getByLabel("E-mail").fill("carlos@palcoeluz.com.br");
    await page
      .getByLabel("Senha Inicial (mínimo 8 caracteres)")
      .fill("Carlos#2026");
    await page.getByRole("button", { name: "Salvar" }).click();

    await expect(
      page.getByText("Usuário cadastrado com sucesso!"),
    ).toBeVisible();
    await expect(page.getByText("Carlos Comercial")).toBeVisible();
    await expect(page.getByText("carlos@palcoeluz.com.br")).toBeVisible();

    // 7. Navega para Perfis de Acesso (/configuracoes/perfis)
    await page
      .getByRole("link", { name: "Perfis de Acesso", exact: true })
      .click();
    await expect(page).toHaveURL(/\/configuracoes\/perfis$/);
    await expect(page.getByTestId("role-card-proprietario")).toBeVisible();
    await expect(page.getByTestId("role-card-comercial")).toBeVisible();
    await expect(page.getByTestId("role-card-estoque")).toBeVisible();
    await expect(page.getByTestId("role-card-tecnico")).toBeVisible();
    await expect(page.getByTestId("role-card-financeiro")).toBeVisible();

    // 8. Navega para Aparência (/configuracoes/aparencia) e altera o nome da empresa
    await page.getByRole("link", { name: "Aparência", exact: true }).click();
    await expect(page).toHaveURL(/\/configuracoes\/aparencia$/);

    const nameInput = page.getByLabel("Nome da Empresa");
    await nameInput.fill("Luz & Som Master");
    await page.getByRole("button", { name: "Salvar Alterações" }).click();

    await expect(
      page.getByText("Identidade visual salva e aplicada com sucesso!"),
    ).toBeVisible();
    await expect(page.locator("h1")).toContainText("Luz & Som Master");
  });
});
