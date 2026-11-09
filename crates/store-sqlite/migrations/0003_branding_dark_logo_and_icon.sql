-- Migração 0003: Suporte a logo para tema escuro e ícone quadrado da marca
ALTER TABLE tenant_branding ADD COLUMN logo_dark_url TEXT;
ALTER TABLE tenant_branding ADD COLUMN icon_url TEXT;

