import type {
  AppStatus,
  InitialSetupInput,
  InitialSetupResult,
  LoginInput,
  RecoverPasswordInput,
  SessionInfo,
} from "./types";

/**
 * Contrato da camada de dados para configuração inicial (modo de operação) e autenticação.
 * As telas interagem exclusivamente com esta porta.
 */
export interface AuthRepository {
  getAppStatus(): Promise<AppStatus>;
  completeInitialSetup(input: InitialSetupInput): Promise<InitialSetupResult>;
  login(input: LoginInput): Promise<SessionInfo>;
  logout(): Promise<void>;
  getSession(): Promise<SessionInfo | null>;
  recoverPasswordWithKey(input: RecoverPasswordInput): Promise<void>;
}
