-- Migração 0010: Controle de época de autenticação para invalidação de sessão (Fase 2S / ADR-0016)
ALTER TABLE app_user ADD COLUMN auth_epoch INTEGER NOT NULL DEFAULT 0;
