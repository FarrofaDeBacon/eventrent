import { test, expect } from "@playwright/test";

test.describe("Fluxo E2E — Gestão de Unidades Físicas e Estoque Multidepósito (Fase 2B)", () => {
  test("deve gerenciar depósitos, tombar unidade física com patrimônio sequencial e consultar livro-razão", async ({
    page,
  }) => {
    // 1. Setup inicial da empresa e proprietário
    await page.goto("/");
    await expect(page).toHaveURL(/\/setup$/);

    await page.getByLabel("Nome da sua Empresa").fill("Som & Luz Eventos SP");
    await page.getByLabel("Seu Nome Completo").fill("Fernando Proprietário");
    await page
      .getByLabel("E-mail de Acesso")
      .fill("fernando@somluzeventos.com.br");
    await page
      .getByLabel("Senha de Acesso (mínimo 8 caracteres)")
      .fill("FernandoSeguro#2026");

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

    // 2. Acesso à página de Estoque & Depósitos pelo menu lateral
    const inventoryNav = page.getByRole("link", {
      name: "Estoque & Depósitos",
      exact: true,
    });
    await expect(inventoryNav).toBeVisible();
    await inventoryNav.click();
    await expect(page).toHaveURL(/\/inventory$/);

    // 3. Verificação do Galpão Principal padrão provisionado na tabela de depósitos
    await expect(
      page.getByRole("heading", { name: "Gestão de Estoque & Depósitos" }),
    ).toBeVisible();
    const warehousesTable = page.getByTestId("table-warehouses");
    await expect(warehousesTable).toBeVisible();
    await expect(warehousesTable.getByText("Galpão Principal")).toBeVisible();
    await expect(warehousesTable.getByText("Padrão")).toBeVisible();

    // 4. Cadastrar novo depósito via modal
    const newWarehouseBtn = page.getByTestId("btn-new-warehouse");
    await expect(newWarehouseBtn).toBeVisible();
    await newWarehouseBtn.click();

    await page.locator("#new-wh-name").fill("Galpão Secundário - Zona Sul");
    await page.locator("#new-wh-code").fill("GALP-ZS");
    await page
      .locator("#new-wh-desc")
      .fill("Depósito avançado para eventos no litoral e zona sul");
    await page.locator("#new-wh-city").fill("São Paulo");
    await page.locator("#new-wh-state").fill("SP");
    await page.getByRole("button", { name: "Criar Depósito" }).click();

    await expect(page.getByText("Depósito criado com sucesso!")).toBeVisible();
    await expect(
      warehousesTable.getByText("Galpão Secundário - Zona Sul"),
    ).toBeVisible();
    await expect(warehousesTable.getByText("GALP-ZS")).toBeVisible();

    // 5. Cadastrar um modelo de equipamento serializado no catálogo
    const catalogNav = page.getByRole("link", {
      name: "Equipamentos",
      exact: true,
    });
    await catalogNav.click();
    await expect(page).toHaveURL(/\/catalog$/);

    // Carregar categorias sugeridas para ter opções disponíveis
    await page.getByRole("link", { name: "Categorias e Atributos" }).click();
    await expect(page).toHaveURL(/\/catalog\/categories$/);
    await page
      .getByRole("button", { name: "Carregar categorias sugeridas" })
      .click();
    await expect(page.getByText(/Sugestões carregadas:/)).toBeVisible();

    await page.getByLabel("Voltar para catálogo").click();
    await expect(page).toHaveURL(/\/catalog$/);

    await page.getByRole("link", { name: "Novo modelo" }).click();
    await expect(page).toHaveURL(/\/catalog\/new$/);

    await page.getByLabel("Nome do modelo *").fill("Pioneer CDJ-3000");

    const catSelect = page.getByLabel("Categoria *");
    const optionVal = await catSelect
      .locator("option")
      .nth(1)
      .getAttribute("value");
    expect(optionVal).toBeTruthy();
    await catSelect.selectOption(optionVal!);

    await page.getByRole("button", { name: "Salvar modelo" }).click();
    await expect(page).toHaveURL(/\/catalog\/[a-zA-Z0-9_-]+$/);

    // 6. Navegar para a aba "Unidades & Estoque" na ficha do modelo
    const inventoryTab = page.getByTestId("tab-model-inventory");
    await expect(inventoryTab).toBeVisible();
    await inventoryTab.click();

    // 7. Tombar a primeira unidade física (deve gerar EQ000001)
    const newUnitBtn = page.getByTestId("btn-add-unit");
    await expect(newUnitBtn).toBeVisible();
    await newUnitBtn.click();

    await page.locator("#new-unit-serial").fill("PIONEER-SN-8822");
    await page
      .locator("#new-unit-notes")
      .fill("Equipamento recebido novo na caixa lacrada.");
    await page
      .getByRole("button", { name: "Cadastrar e Gerar Patrimônio" })
      .click();

    await expect(page.getByText("EQ000001")).toBeVisible();
    await expect(page.getByText("PIONEER-SN-8822")).toBeVisible();
    await expect(page.getByText("Disponível")).toBeVisible();

    // 8. Voltar para a rota global /inventory
    await inventoryNav.click();
    await expect(page).toHaveURL(/\/inventory$/);

    // 9. Verificar unidade física na aba global "Unidades Físicas"
    const unitsTab = page.getByTestId("tab-units");
    await unitsTab.click();

    const unitsTable = page.getByTestId("table-units");
    await expect(unitsTable).toBeVisible();
    await expect(unitsTable.getByText("EQ000001")).toBeVisible();
    await expect(unitsTable.getByText("PIONEER-SN-8822")).toBeVisible();
    await expect(unitsTable.getByText("Galpão Principal")).toBeVisible();

    // 10. Consultar e registrar movimentação na aba "Livro-Razão & Movimentações"
    const movementsTab = page.getByTestId("tab-movements");
    await movementsTab.click();

    const newMovBtn = page.getByTestId("btn-new-movement");
    await expect(newMovBtn).toBeVisible();
    await newMovBtn.click();

    await page
      .locator("#mov-reason")
      .fill("Compra de estoque inicial via NF-5544");
    await page.locator("#mov-ref").fill("NF-5544");
    await page
      .getByRole("button", { name: "Registrar no Livro-Razão" })
      .click();

    const movementsTable = page.getByTestId("table-movements");
    await expect(movementsTable).toBeVisible();
    await expect(
      movementsTable.getByText("Compra de estoque inicial via NF-5544"),
    ).toBeVisible();
  });
});
