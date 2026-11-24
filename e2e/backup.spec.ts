import { test, expect } from "@playwright/test";

test.describe("Fluxo E2E — Configuração, Execução e Agendamento de Restauração de Backup", () => {
  test("deve configurar retenção, executar backup imediato e abrir modal de restauração com confirmação bloqueante", async ({
    page,
  }) => {
    // 1. Configuração inicial da empresa e proprietário
    await page.goto("/");
    await expect(page).toHaveURL(/\/setup$/);

    await page.getByLabel("Nome da sua Empresa").fill("Locadora Áudio & Vídeo");
    await page.getByLabel("Seu Nome Completo").fill("Roberto Dono");
    await page
      .getByLabel("E-mail de Acesso")
      .fill("roberto@audioevideo.com.br");
    await page
      .getByLabel("Senha de Acesso (mínimo 8 caracteres)")
      .fill("Segredo#2026");

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

    // 2. Navegação para Backup e Restauração via menu lateral
    const backupLink = page.getByRole("link", {
      name: "Backup e Restauração",
      exact: true,
    });
    await expect(backupLink).toBeVisible();
    await backupLink.click();
    await expect(page).toHaveURL(/\/settings\/backup$/);

    // 3. Verificação de elementos essenciais da tela
    await expect(
      page.getByRole("heading", { name: "Backup e Restauração" }),
    ).toBeVisible();
    await expect(
      page.getByText("Aviso de Privacidade e Conformidade LGPD"),
    ).toBeVisible();

    // 4. Configuração de pasta e retenção
    const folderInput = page.locator("#destination-folder");
    await expect(folderInput).toBeVisible();
    await folderInput.fill("D:\\EventRent-Backups");

    await page.getByRole("button", { name: "Salvar Configurações" }).click();
    await expect(
      page.getByText("Configurações de backup salvas com sucesso."),
    ).toBeVisible();

    // 5. Execução manual de backup imediato
    const runBackupBtn = page.getByRole("button", {
      name: "Fazer Backup Agora",
    });
    await expect(runBackupBtn).toBeVisible();
    await runBackupBtn.click();

    // Feedback de sucesso
    await expect(page.getByText(/Backup concluído:/)).toBeVisible();

    // 6. Tabela de backups encontrados na pasta
    const restoreButtons = page.getByRole("button", { name: "Restaurar..." });
    await expect(restoreButtons.first()).toBeVisible();
    await restoreButtons.first().click();

    // 7. Modal bloqueante de confirmação com palavra-chave RESTAURAR
    await expect(
      page.getByRole("heading", { name: "Confirmar Restauração a Frio" }),
    ).toBeVisible();
    await expect(
      page.getByText(
        "A restauração substituirá integralmente todos os dados cadastrados no aplicativo",
      ),
    ).toBeVisible();

    const confirmButton = page.getByRole("button", {
      name: "Confirmar e Agendar Restauração",
    });
    await expect(confirmButton).toBeDisabled();

    const passwordInput = page.locator("#restore-password-input");
    await passwordInput.fill("SenhaForte#123");

    const confirmInput = page.locator("#restore-confirm-input");
    await confirmInput.fill("restaurar");
    await expect(confirmButton).toBeDisabled();

    await confirmInput.fill("RESTAURAR");
    await expect(confirmButton).toBeEnabled();

    await confirmButton.click();
    await expect(
      page.getByText(
        "Restauração agendada com sucesso. Feche e reinicie o EventRent para concluir.",
      ),
    ).toBeVisible();
  });
});
