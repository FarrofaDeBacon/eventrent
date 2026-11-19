import { expect, test } from "@playwright/test";

// 1x1 PNG transparente válido para upload e colagem
const VALID_1X1_PNG_BYTES = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
  "base64",
);

const VALID_PDF_BYTES = Buffer.from(
  "%PDF-1.7\n1 0 obj\n<<>>\nendobj\n%%EOF",
  "utf-8",
);

test.describe("E2E — Mídia de Modelos de Equipamento: Fotos, Documentos e Importação em Lote (Fase 2M)", () => {
  test("deve anexar foto, colar imagem via Ctrl+V, definir principal, reordenar, anexar PDF, importar fotos em lote e respeitar modo somente leitura para Técnico", async ({
    page,
  }) => {
    // 1. Setup inicial como Proprietário
    await page.goto("/");
    await expect(page).toHaveURL(/\/setup$/);

    await page.getByLabel("Nome da sua Empresa").fill("AudioPro Locação");
    await page.getByLabel("Seu Nome Completo").fill("Carlos Proprietário");
    await page.getByLabel("E-mail de Acesso").fill("carlos@audiopro.com.br");
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

    // 2. Carregar categorias sugeridas (incluindo DJ e Acessórios da Tarefa 0.7) e criar modelo CDJ-3000
    await page.getByRole("link", { name: "Equipamentos", exact: true }).click();
    await expect(page).toHaveURL(/\/catalog$/);

    await page.getByRole("link", { name: "Categorias e Atributos" }).click();
    await expect(page).toHaveURL(/\/catalog\/categories$/);

    await page
      .getByRole("button", { name: "Carregar categorias sugeridas" })
      .click();
    await expect(page.getByText(/Sugestões carregadas:/)).toBeVisible();
    await expect(
      page.getByRole("button", { name: /↳ Players \(CDJ\)/ }),
    ).toBeVisible();

    await page.getByLabel("Voltar para catálogo").click();
    await page.getByRole("link", { name: "Novo modelo" }).click();
    await expect(page).toHaveURL(/\/catalog\/new$/);

    await page.getByLabel("Nome do modelo *").fill("CDJ-3000");
    const catSelect = page.getByLabel("Categoria *");
    const cdjCatVal = await catSelect
      .locator("option", { hasText: "Players (CDJ)" })
      .getAttribute("value");
    expect(cdjCatVal).toBeTruthy();
    await catSelect.selectOption(cdjCatVal!);

    await page.getByLabel("SKU interno").fill("DJ-CDJ-3000");
    await page.getByLabel("Código do modelo").fill("CDJ-3000");
    await page.getByLabel("EAN / Código de barras").fill("4573201242006");
    await page.getByRole("button", { name: "Salvar modelo" }).click();
    await expect(page).toHaveURL(/\/catalog\/model-/);

    // 3. Aba Fotos: anexar foto por seletor de arquivo
    await page.getByTestId("tab-model-photos").click();
    await expect(page.getByTestId("model-photos-section")).toBeVisible();

    await page.getByTestId("photo-file-input").setInputFiles({
      name: "painel-frontal.png",
      mimeType: "image/png",
      buffer: VALID_1X1_PNG_BYTES,
    });

    await expect(page.getByTestId("photo-gallery-card")).toHaveCount(1);
    await expect(page.getByTestId("primary-photo-badge")).toHaveCount(1);
    await expect(page.getByTestId("photo-final-size-summary")).toContainText(
      "Tamanho final:",
    );

    // Verifica que o atributo alt da imagem é o nome do modelo ("CDJ-3000")
    await expect(
      page.getByTestId("photo-gallery-card").locator('img[alt="CDJ-3000"]'),
    ).toHaveCount(1);

    // 4. Colar imagem da área de transferência (Ctrl+V)
    await page.evaluate((base64Png) => {
      const bin = atob(base64Png);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) {
        bytes[i] = bin.charCodeAt(i);
      }
      const file = new File([bytes], "colada-clipboard.png", {
        type: "image/png",
      });
      const dt = new DataTransfer();
      dt.items.add(file);
      const evt = new ClipboardEvent("paste", {
        clipboardData: dt,
        bubbles: true,
        cancelable: true,
      });
      window.dispatchEvent(evt);
    }, VALID_1X1_PNG_BYTES.toString("base64"));

    await expect(page.getByTestId("photo-gallery-card")).toHaveCount(2);
    await expect(
      page.getByText("colada-clipboard", { exact: true }),
    ).toBeVisible();

    // 5. Definir a segunda foto como principal e reordenar
    await page
      .getByRole("button", { name: "Definir principal" })
      .first()
      .click();
    await expect(page.getByText("Foto principal definida.")).toBeVisible();

    await page
      .getByRole("button", { name: "Mover foto para frente" })
      .first()
      .click();
    await expect(page.getByText("Ordem das fotos atualizada.")).toBeVisible();

    // 6. Aba Documentos: anexar PDF
    await page.getByTestId("tab-model-documents").click();
    await expect(page.getByTestId("model-documents-section")).toBeVisible();

    await page.getByTestId("document-file-input").setInputFiles({
      name: "manual-cdj-3000.pdf",
      mimeType: "application/pdf",
      buffer: VALID_PDF_BYTES,
    });
    await page.getByLabel("Título *").fill("Manual de Operação CDJ-3000");
    await page.getByLabel("Versão (opcional)").fill("Rev 2.1");
    await page.getByRole("button", { name: "Anexar PDF" }).click();

    await expect(page.getByTestId("document-table-row")).toHaveCount(1);
    await expect(page.getByText("Manual de Operação CDJ-3000")).toBeVisible();
    await expect(page.getByText("Rev 2.1")).toBeVisible();

    // 7. Importação em lote de fotos em /catalog/import-photos
    await page.getByLabel("Voltar para catálogo").click();
    await expect(page).toHaveURL(/\/catalog$/);
    await expect(page.getByTestId("catalog-list-thumb")).toHaveCount(1);

    await page.getByRole("link", { name: "Importar fotos em lote" }).click();
    await expect(page).toHaveURL(/\/catalog\/import-photos$/);

    await page.getByTestId("batch-photo-input").setInputFiles([
      {
        name: "DJ-CDJ-3000.png",
        mimeType: "image/png",
        buffer: VALID_1X1_PNG_BYTES,
      },
      {
        name: "foto-sem-codigo.png",
        mimeType: "image/png",
        buffer: VALID_1X1_PNG_BYTES,
      },
    ]);

    await expect(page.getByTestId("batch-import-row")).toHaveCount(2);
    const statuses = page.getByTestId("batch-row-status");
    await expect(statuses.nth(0)).toContainText("Casado");
    await expect(statuses.nth(0)).toContainText("SKU");
    await expect(statuses.nth(1)).toContainText("Sem correspondência");

    // Corrige manualmente o segundo arquivo selecionando o modelo CDJ-3000
    const secondSelect = page.getByTestId("batch-row-model-select").nth(1);
    const cdjModelId = await secondSelect
      .locator("option", { hasText: "CDJ-3000" })
      .getAttribute("value");
    expect(cdjModelId).toBeTruthy();
    await secondSelect.selectOption(cdjModelId!);
    await expect(statuses.nth(1)).toContainText("Casado");

    await page.getByTestId("confirm-batch-import-btn").click();
    await expect(page.getByTestId("batch-summary-counts")).toContainText(
      "2 sucesso(s)",
    );

    // 8. Criar usuário Técnico e validar modo somente leitura nas abas Fotos e Documentos
    await page.getByRole("link", { name: "Usuários", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/users$/);
    await page.getByTestId("new-user-btn").click();
    await page.getByLabel("Nome Completo").fill("Lucas Técnico");
    await page.getByLabel("E-mail").fill("lucas@audiopro.com.br");
    await page
      .getByLabel("Senha Inicial (mínimo 8 caracteres)")
      .fill("Tecnico#2026");
    await page.getByLabel("Perfil de Acesso").selectOption("role-technician");
    await page.getByRole("button", { name: "Salvar" }).click();
    await expect(page.getByText("lucas@audiopro.com.br")).toBeVisible();

    await page.getByRole("button", { name: "Sair" }).click();
    await expect(page).toHaveURL(/\/login$/);
    await page.getByLabel("E-mail").fill("lucas@audiopro.com.br");
    await page.getByLabel("Senha").fill("Tecnico#2026");
    await page.getByRole("button", { name: "Entrar" }).click();
    await expect(page).toHaveURL(/\/$/);

    await page.getByRole("link", { name: "Equipamentos", exact: true }).click();
    await expect(page).toHaveURL(/\/catalog$/);
    await expect(
      page.getByRole("link", { name: "Importar fotos em lote" }),
    ).toHaveCount(0);

    await page.getByText("CDJ-3000", { exact: true }).click();
    await page.getByTestId("tab-model-photos").click();
    await expect(page.getByTestId("photo-gallery-card")).toHaveCount(4);
    await expect(page.getByTestId("photo-drop-paste-zone")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Definir principal" }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Remover foto" }),
    ).toHaveCount(0);

    await page.getByTestId("tab-model-documents").click();
    await expect(page.getByTestId("document-table-row")).toHaveCount(1);
    await expect(
      page.getByRole("button", { name: "Abrir", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Salvar como..." }),
    ).toBeVisible();
    await expect(page.getByTestId("document-file-input")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Remover documento" }),
    ).toHaveCount(0);
  });
});
