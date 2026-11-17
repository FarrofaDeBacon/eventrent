import { test, expect } from "@playwright/test";

test.describe("E2E — Módulo de Fornecedores e Papéis (Fase 1C)", () => {
  test("deve criar fornecedor de sublocação, bloquear perfil Técnico e dar somente leitura ao Comercial", async ({
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

    // 2. Navega para Fornecedores (/suppliers) e cria fornecedor de Sublocação
    await page.getByRole("link", { name: "Fornecedores", exact: true }).click();
    await expect(page).toHaveURL(/\/suppliers$/);
    await expect(
      page.getByRole("heading", { name: "Fornecedores" }),
    ).toBeVisible();

    await page.getByTestId("new-customer-btn").click();
    await expect(page).toHaveURL(/\/customers\/new\?role=supplier$/);

    // Papel Fornecedor já vem marcado e chips de categoria visíveis
    await expect(page.getByTestId("role-supplier-checkbox")).toBeChecked();
    await expect(page.getByTestId("role-customer-checkbox")).not.toBeChecked();
    await expect(page.getByTestId("supplier-categories-section")).toBeVisible();

    await page.getByTestId("supplier-category-sublocacao").click();
    await page
      .getByLabel("Razão Social *")
      .fill("Mega Sublocação Palco & Som Ltda");
    await page
      .locator("select[aria-label='Tipo de documento']")
      .selectOption("none");
    await page.locator("#contact-name-0").fill("Roberto Sublocação");
    await page.locator("#contact-email-0").fill("roberto@megasubloca.com.br");
    await page.locator("#contact-phone-0").fill("11988887777");
    await page.getByTestId("save-customer-btn").click();

    await expect(
      page.getByText("Cliente cadastrado com sucesso!"),
    ).toBeVisible();
    await expect(
      page.getByText("Fornecedor", { exact: true }).first(),
    ).toBeVisible();

    // Confirma visibilidade nas abas de /customers (Fornecedores e Todos)
    await page.getByRole("link", { name: "Clientes", exact: true }).click();
    await expect(page).toHaveURL(/\/customers$/);
    await expect(
      page.getByText("Mega Sublocação Palco & Som Ltda"),
    ).not.toBeVisible();

    await page.getByTestId("tab-suppliers").click();
    await expect(
      page.getByText("Mega Sublocação Palco & Som Ltda"),
    ).toBeVisible();

    await page.getByTestId("tab-all").click();
    await expect(
      page.getByText("Mega Sublocação Palco & Som Ltda"),
    ).toBeVisible();

    // 3. Cria usuários Técnico e Comercial
    await page.getByRole("link", { name: "Usuários", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/users$/);

    await page.getByTestId("new-user-btn").click();
    await page.getByLabel("Nome Completo").fill("Lucas Técnico");
    await page.getByLabel("E-mail").fill("lucas@eventospalco.com.br");
    await page
      .getByLabel("Senha Inicial (mínimo 8 caracteres)")
      .fill("Lucas#2026");
    await page.getByLabel("Perfil de Acesso").selectOption("role-technician");
    await page.getByRole("button", { name: "Salvar" }).click();
    await expect(
      page.getByText("Usuário cadastrado com sucesso!").first(),
    ).toBeVisible();
    await expect(page.getByText("Lucas Técnico")).toBeVisible();

    await page.getByTestId("new-user-btn").click();
    await page.getByLabel("Nome Completo").fill("Carla Comercial");
    await page.getByLabel("E-mail").fill("carla@eventospalco.com.br");
    await page
      .getByLabel("Senha Inicial (mínimo 8 caracteres)")
      .fill("Carla#2026");
    await page.getByLabel("Perfil de Acesso").selectOption("role-commercial");
    await page.getByRole("button", { name: "Salvar" }).click();
    await expect(
      page.getByText("Usuário cadastrado com sucesso!").first(),
    ).toBeVisible();
    await expect(page.getByText("Carla Comercial")).toBeVisible();

    // 4. Perfil Técnico: sem acesso a Clientes nem Fornecedores
    await page.getByRole("button", { name: "Sair" }).click();
    await expect(page).toHaveURL(/\/login$/);
    await page.getByLabel("E-mail").fill("lucas@eventospalco.com.br");
    await page.getByLabel("Senha").fill("Lucas#2026");
    await page.getByRole("button", { name: "Entrar" }).click();
    await expect(page).toHaveURL(/\/$/);

    await expect(
      page.getByRole("link", { name: "Clientes", exact: true }),
    ).not.toBeVisible();
    await expect(
      page.getByRole("link", { name: "Fornecedores", exact: true }),
    ).not.toBeVisible();

    await page.goto("/suppliers");
    await expect(page).toHaveURL(/\/$/);
    await page.goto("/customers");
    await expect(page).toHaveURL(/\/$/);

    // 5. Perfil Comercial: somente leitura em Fornecedores (lê, mas não cria nem edita)
    await page.getByRole("button", { name: "Sair" }).click();
    await expect(page).toHaveURL(/\/login$/);
    await page.getByLabel("E-mail").fill("carla@eventospalco.com.br");
    await page.getByLabel("Senha").fill("Carla#2026");
    await page.getByRole("button", { name: "Entrar" }).click();
    await expect(page).toHaveURL(/\/$/);

    await page.getByRole("link", { name: "Fornecedores", exact: true }).click();
    await expect(page).toHaveURL(/\/suppliers$/);
    await expect(page.getByTestId("new-customer-btn")).not.toBeVisible();

    await page.getByText("Mega Sublocação Palco & Som Ltda").click();
    await expect(page).toHaveURL(/\/customers\/cust-/);
    await expect(page.getByTestId("edit-customer-btn")).not.toBeVisible();
  });
});
