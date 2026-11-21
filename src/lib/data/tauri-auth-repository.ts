import { invoke } from "@tauri-apps/api/core";
import type { AuthRepository } from "./auth-repository";
import type {
  AppStatus,
  InitialSetupInput,
  InitialSetupResult,
  LoginInput,
  RecoverPasswordInput,
  RecoverPasswordResult,
  RegenerateRecoveryKeyInput,
  RegenerateRecoveryKeyResult,
  SessionInfo,
} from "./types";

export class TauriAuthRepository implements AuthRepository {
  async getAppStatus(): Promise<AppStatus> {
    return await invoke<AppStatus>("get_app_status");
  }

  async completeInitialSetup(
    input: InitialSetupInput,
  ): Promise<InitialSetupResult> {
    return await invoke<InitialSetupResult>("complete_initial_setup", {
      input,
    });
  }

  async login(input: LoginInput): Promise<SessionInfo> {
    return await invoke<SessionInfo>("login", { input });
  }

  async logout(): Promise<void> {
    await invoke<void>("logout");
  }

  async getSession(): Promise<SessionInfo | null> {
    return await invoke<SessionInfo | null>("get_current_session");
  }

  async recoverPasswordWithKey(
    input: RecoverPasswordInput,
  ): Promise<RecoverPasswordResult> {
    return await invoke<RecoverPasswordResult>("recover_password", { input });
  }

  async regenerateRecoveryKey(
    input: RegenerateRecoveryKeyInput,
  ): Promise<RegenerateRecoveryKeyResult> {
    return await invoke<RegenerateRecoveryKeyResult>(
      "regenerate_recovery_key",
      {
        input,
      },
    );
  }
}
