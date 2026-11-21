import { test, expect } from "@playwright/test";

test.describe("Recuperação de Senha e Regeneração de Chave (S1)", () => {
  test("deve recuperar senha exibindo nova chave em modal bloqueante com confirmação digitada, fazer login e regenerar chave autenticada em Minha Conta", async ({
    page,
  }) => {
    // 1. Configuração inicial
    await page.goto("/");
    await expect(page).toHaveURL(/\/setup$/);

    await page.getByLabel("Nome da sua Empresa").fill("Som & Luz Auditoria");
    await page.getByLabel("Seu Nome Completo").fill("Carlos Diretor");
    await page.getByLabel("E-mail de Acesso").fill("carlos@someluz.com.br");
    await page
      .getByLabel("Senha de Acesso (mínimo 8 caracteres)")
      .fill("SenhaOriginal#2026");

    await page
      .getByRole("button", { name: "Concluir Configuração Inicial" })
      .click();

    // Captura chave inicial do setup
    const recoveryKeyEl = page.getByTestId("recovery-key-value");
    await expect(recoveryKeyEl).toBeVisible();
    const initialKey = (await recoveryKeyEl.innerText()).trim();

    await page
      .getByLabel(
        "Anotei e guardei minha Chave de Recuperação em um local seguro",
      )
      .check();
    await page.getByRole("button", { name: "Entrar no Sistema" }).click();

    // 2. Faz logout para testar fluxo de recuperação na tela de Login
    await expect(page).toHaveURL(/\/$/);
    await page.getByRole("button", { name: "Sair" }).click();
    await expect(page).toHaveURL(/\/login$/);

    // 3. Abre modal de recuperação de senha
    await page
      .getByRole("button", {
        name: "Esqueci minha senha (usar Chave de Recuperação)",
      })
      .click();

    await page.locator("#recovery-email").fill("carlos@someluz.com.br");
    await page.locator("#recovery-key").fill(initialKey);
    await page
      .locator("#recovery-new-password")
      .fill("NovaSenhaRecuperada#2026");

    await page.getByRole("button", { name: "Redefinir Senha" }).click();

    // 4. Modal bloqueante de nova chave de recuperação (S1.2)
    const newKeyDisplay = page.getByTestId("new-recovery-key-display");
    await expect(newKeyDisplay).toBeVisible();
    const rotatedKey = (await newKeyDisplay.innerText()).trim();
    expect(rotatedKey).not.toBe(initialKey);

    // Botões de ação do modal
    await expect(page.getByTestId("copy-recovery-key-btn")).toBeVisible();
    await expect(page.getByTestId("download-recovery-key-btn")).toBeVisible();
    await expect(page.getByTestId("print-recovery-key-btn")).toBeVisible();

    // Botão de continuar deve estar desabilitado antes da confirmação digitada
    const continueBtn = page.getByTestId("finish-recovery-modal-btn");
    await expect(continueBtn).toBeDisabled();

    // Digita texto incompleto ou errado
    const confirmInput = page.getByTestId("confirm-recovery-input");
    await confirmInput.fill("guardei");
    await expect(continueBtn).toBeDisabled();

    // Digita a frase exata "Guardei a chave"
    await confirmInput.fill("Guardei a chave");
    await expect(continueBtn).toBeEnabled();
    await continueBtn.click();

    // Modal fechado
    await expect(newKeyDisplay).not.toBeVisible();

    // 5. Login com a nova senha redefinida
    await page.locator("#login-email").fill("carlos@someluz.com.br");
    await page.locator("#login-password").fill("NovaSenhaRecuperada#2026");
    await page.getByRole("button", { name: "Entrar" }).click();

    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByTestId("header-user-info")).toContainText(
      "Carlos Diretor",
    );

    // 6. Navega para "Minha Conta" (/settings/account) (S1.3)
    await page.goto("/settings/account");
    await expect(
      page.getByRole("heading", { name: "Minha Conta" }),
    ).toBeVisible();
    await expect(page.getByText("carlos@someluz.com.br")).toBeVisible();

    // Inicia regeneração autenticada
    await page.getByTestId("regenerate-key-btn").click();

    // Tenta com senha errada
    await page.getByTestId("current-password-input").fill("SenhaErrada#9999");
    await page.getByTestId("confirm-regenerate-btn").click();
    await expect(page.getByText(/incorreta/i).first()).toBeVisible();

    // Confirma com a senha correta
    await page
      .getByTestId("current-password-input")
      .fill("NovaSenhaRecuperada#2026");
    await page.getByTestId("confirm-regenerate-btn").click();

    // Modal bloqueante de nova chave regenerada
    await expect(newKeyDisplay).toBeVisible();
    const regeneratedKey = (await newKeyDisplay.innerText()).trim();
    expect(regeneratedKey).not.toBe(rotatedKey);

    await confirmInput.fill("Guardei a chave");
    await expect(continueBtn).toBeEnabled();
    await continueBtn.click();

    await expect(newKeyDisplay).not.toBeVisible();
  });
});
