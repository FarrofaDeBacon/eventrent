import { test, expect } from "@playwright/test";

function generateAlphaCnpj(base12: string): string {
  const clean = base12.toUpperCase();
  const charValue = (ch: string) => ch.charCodeAt(0) - 48;
  const w1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
  let s1 = 0;
  for (let i = 0; i < 12; i++) s1 += charValue(clean[i]) * w1[i];
  const r1 = s1 % 11;
  const dv1 = r1 < 2 ? 0 : 11 - r1;

  const w2 = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
  let s2 = 0;
  for (let i = 0; i < 12; i++) s2 += charValue(clean[i]) * w2[i];
  s2 += dv1 * w2[12];
  const r2 = s2 % 11;
  const dv2 = r2 < 2 ? 0 : 11 - r2;

  return `${clean}${dv1}${dv2}`;
}

test.describe("E2E — Módulo de Clientes (Fase 1B.2)", () => {
  test("deve navegar para clientes, filtrar, cadastrar PJ com CNPJ alfanumérico, editar, buscar via Ctrl+K e excluir", async ({
    page,
  }) => {
    // 1. Setup inicial
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

    // 2. Navega para Clientes via Sidebar
    await page.getByRole("link", { name: "Clientes", exact: true }).click();
    await expect(page).toHaveURL(/\/customers$/);
    await expect(
      page.getByText("Produtora Horizonte Eventos Ltda"),
    ).toBeVisible();

    // 3. Testa filtro por Pessoa Física e busca sem acento
    await page.getByTestId("filter-individual").click();
    await expect(page.getByText("Carlos Eduardo da Silva")).toBeVisible();
    await expect(
      page.getByText("Produtora Horizonte Eventos Ltda"),
    ).not.toBeVisible();

    await page.getByTestId("filter-all").click();
    await page.getByTestId("customers-search-input").fill("horizonte");
    await expect(
      page.getByText("Produtora Horizonte Eventos Ltda"),
    ).toBeVisible();

    // 4. Cadastro de Novo Cliente PJ com CNPJ Alfanumérico (IN RFB nº 2.229/2025)
    await page.getByTestId("new-customer-btn").click();
    await expect(page).toHaveURL(/\/customers\/new$/);

    const alphaCnpj = generateAlphaCnpj("12ABC34501DE");

    await page
      .getByLabel("Razão Social *")
      .fill("Mega Áudio & Luz Alfanum Ltda");
    await page.getByLabel("Nome Fantasia").fill("Mega Áudio Alfa");
    await page.getByLabel("CNPJ (Numérico ou Alfanumérico) *").fill(alphaCnpj);
    await expect(page.getByText("Documento válido")).toBeVisible();

    // Preenche Contato #1
    await page.locator("#contact-name-0").fill("Fernanda Eventos");
    await page.locator("#contact-email-0").fill("fernanda@megaaudio.com.br");
    await page.locator("#contact-phone-0").fill("11988887777");

    // Preenche Endereço #1
    await page.locator("#address-label-0").fill("Galpão Principal");
    await page.locator("#address-cep-0").fill("04538132");
    await page.locator("#address-street-0").fill("Av. Brigadeiro Faria Lima");
    await page.locator("#address-number-0").fill("3477");
    await page.locator("#address-neighborhood-0").fill("Itaim Bibi");
    await page.locator("#address-city-0").fill("São Paulo");
    await page.locator("#address-state-0").selectOption("SP");

    await page.getByTestId("save-customer-btn").click();

    await expect(
      page.getByText("Cliente cadastrado com sucesso!"),
    ).toBeVisible();
    await expect(page).toHaveURL(/\/customers\/cust-/);
    await expect(
      page.getByRole("heading", { name: "Mega Áudio & Luz Alfanum Ltda" }),
    ).toBeVisible();

    // 5. Edição do cliente
    await page.getByTestId("edit-customer-btn").click();
    await page
      .getByLabel("Razão Social *")
      .fill("Mega Áudio & Luz Atualizada Ltda");
    await page.getByTestId("save-customer-btn").click();
    await expect(
      page.getByText("Cliente atualizado com sucesso!"),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Mega Áudio & Luz Atualizada Ltda" }),
    ).toBeVisible();

    // 6. Busca rápida via Command Palette (Ctrl+K)
    await page.goto("/");
    await expect(
      page.getByRole("heading", {
        name: /EventRent — Gestão de Locação para Eventos/i,
      }),
    ).toBeVisible();
    await page.keyboard.press("Control+k");
    const cmdInput = page.getByLabel(
      "Digite um comando ou pesquise (Ctrl+K)...",
    );
    await expect(cmdInput).toBeVisible();
    await cmdInput.fill("mega audio");
    const cmdResult = page.getByRole("button", {
      name: /Mega Áudio & Luz Atualizada Ltda/i,
    });
    await expect(cmdResult).toBeVisible();
    await cmdResult.click();
    await expect(page).toHaveURL(/\/customers\/cust-/);

    // 7. Exclusão lógica (soft delete) com confirmação
    await page.getByTestId("delete-customer-btn").click();
    await page.getByTestId("confirm-delete-customer-btn").click();
    await expect(page.getByText("Cliente excluído com sucesso!")).toBeVisible();
    await expect(page).toHaveURL(/\/customers$/);
    await expect(
      page.getByText("Mega Áudio & Luz Atualizada Ltda"),
    ).not.toBeVisible();
  });
});
