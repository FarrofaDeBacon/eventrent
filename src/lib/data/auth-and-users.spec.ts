import { beforeEach, describe, expect, it } from "vitest";
import { canAccessRoute, hasPermission } from "./index";
import {
  MockAuthRepository,
  resetSharedMockState,
} from "./mock-auth-repository";
import { MockUserRepository } from "./mock-user-repository";
import { DEFAULT_COMPANY_NAME, PERMISSIONS } from "./types";

describe("AuthRepository & UserRepository (Regras de Domínio e Guards)", () => {
  let authRepo: MockAuthRepository;
  let userRepo: MockUserRepository;

  beforeEach(() => {
    resetSharedMockState();
    authRepo = new MockAuthRepository();
    userRepo = new MockUserRepository();
  });

  it("deve iniciar sem proprietário cadastrado e com nome padrão 'Minha empresa'", async () => {
    const status = await authRepo.getAppStatus();
    expect(status.has_owner).toBe(false);
    expect(status.config.setup_completed).toBe(false);
    expect(status.branding?.company_name).toBe(DEFAULT_COMPANY_NAME);
  });

  it("deve recusar o modo nuvem na Fase 1A e concluir o setup no modo local gerando chave de recuperação", async () => {
    await expect(
      authRepo.completeInitialSetup({
        mode: "cloud",
        company_name: "Som & Luz Nuvem",
        owner_name: "Carlos Silva",
        owner_email: "carlos@someluz.com.br",
        owner_password: "SenhaSegura#2026",
      }),
    ).rejects.toThrow(/modo nuvem estará disponível em breve/i);

    const result = await authRepo.completeInitialSetup({
      mode: "local",
      company_name: "Palco Total Locações",
      owner_name: "Marina Costa",
      owner_email: "marina@palcototal.com.br",
      owner_password: "Marina#Password2026",
    });

    expect(result.recovery_key).toMatch(
      /^ER-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/,
    );
    expect(result.branding.company_name).toBe("Palco Total Locações");
    expect(result.session.role_slug).toBe("proprietario");
    expect(hasPermission(result.session, PERMISSIONS.USERS_CREATE)).toBe(true);
  });

  it("deve bloquear temporariamente após 5 falhas de login e permitir recuperar com a chave de recuperação", async () => {
    const setup = await authRepo.completeInitialSetup({
      mode: "local",
      company_name: "AudioPro Eventos",
      owner_name: "Roberto Lima",
      owner_email: "roberto@audiopro.com.br",
      owner_password: "Roberto#Password2026",
    });

    await authRepo.logout();
    expect(await authRepo.getSession()).toBeNull();

    for (let i = 1; i <= 4; i++) {
      await expect(
        authRepo.login({
          email: "roberto@audiopro.com.br",
          password: `Errada${i}`,
        }),
      ).rejects.toThrow(/E-mail ou senha inválidos/i);
    }

    await expect(
      authRepo.login({
        email: "roberto@audiopro.com.br",
        password: "Errada5",
      }),
    ).rejects.toThrow(/bloqueada por 5 minutos/i);

    // Mesmo com a senha certa, continua bloqueada
    await expect(
      authRepo.login({
        email: "roberto@audiopro.com.br",
        password: "Roberto#Password2026",
      }),
    ).rejects.toThrow(/temporariamente bloqueada/i);

    // Recuperação de senha com a Chave de Recuperação desbloqueia e redefine
    await authRepo.recoverPasswordWithKey({
      email: "roberto@audiopro.com.br",
      recovery_key: setup.recovery_key,
      new_password: "RobertoNovaSenha#2026",
    });

    const session = await authRepo.login({
      email: "roberto@audiopro.com.br",
      password: "RobertoNovaSenha#2026",
    });
    expect(session.email).toBe("roberto@audiopro.com.br");
  });

  it("deve proteger o último Proprietário contra desativação/rebaixamento, impedir autodesativação e validar rotas por permissão", async () => {
    const setup = await authRepo.completeInitialSetup({
      mode: "local",
      company_name: "Mega Som",
      owner_name: "Fernanda Souza",
      owner_email: "fernanda@megasom.com.br",
      owner_password: "Fernanda#Password2026",
    });

    const roles = await userRepo.listRoles();
    expect(roles).toHaveLength(5);
    const comercialRole = roles.find((r) => r.slug === "comercial")!;

    // Impede autodesativação
    await expect(
      userRepo.setUserActive(setup.session.user_id, false),
    ).rejects.toThrow(/não pode desativar sua própria conta/i);

    // Impede rebaixar o último Proprietário ativo
    await expect(
      userRepo.updateUser(setup.session.user_id, {
        name: "Fernanda Souza",
        email: "fernanda@megasom.com.br",
        role_id: comercialRole.id,
      }),
    ).rejects.toThrow(/último Proprietário ativo/i);

    // Cria usuário Comercial
    const comercialUser = await userRepo.createUser({
      name: "Lucas Vendas",
      email: "lucas@megasom.com.br",
      password: "Lucas#Password2026",
      role_id: comercialRole.id,
    });
    expect(comercialUser.role_slug).toBe("comercial");

    // Faz login como Comercial e verifica guards de rota e permissão
    await authRepo.logout();
    const comercialSession = await authRepo.login({
      email: "lucas@megasom.com.br",
      password: "Lucas#Password2026",
    });

    expect(canAccessRoute(comercialSession, "/")).toBe(true);
    expect(canAccessRoute(comercialSession, "/configuracoes/aparencia")).toBe(
      true,
    );
    expect(canAccessRoute(comercialSession, "/configuracoes/usuarios")).toBe(
      false,
    );
    expect(canAccessRoute(comercialSession, "/configuracoes/perfis")).toBe(
      false,
    );

    await expect(userRepo.listUsers()).rejects.toThrow(
      /Permissão insuficiente/i,
    );
  });
});
