use crate::db::DbPool;
use crate::domain::branding::{TenantBranding, UpdateBrandingInput};
use crate::error::{CoreError, Result};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

pub struct BrandingRepository;

impl BrandingRepository {
    pub async fn get_by_tenant_id(
        pool: &DbPool,
        tenant_id: &str,
    ) -> Result<Option<TenantBranding>> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, company_name, primary_color, secondary_color,
                   accent_color, logo_url, favicon_url, created_at, updated_at,
                   deleted_at, created_by
            FROM tenant_branding
            WHERE tenant_id = ? AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(pool)
        .await?;

        let branding = row.map(|r| TenantBranding {
            id: r.get("id"),
            tenant_id: r.get("tenant_id"),
            company_name: r.get("company_name"),
            primary_color: r.get("primary_color"),
            secondary_color: r.get("secondary_color"),
            accent_color: r.get("accent_color"),
            logo_url: r.get("logo_url"),
            favicon_url: r.get("favicon_url"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            deleted_at: r.get("deleted_at"),
            created_by: r.get("created_by"),
        });

        Ok(branding)
    }

    pub async fn save(
        pool: &DbPool,
        tenant_id: &str,
        input: &UpdateBrandingInput,
    ) -> Result<TenantBranding> {
        let existing = Self::get_by_tenant_id(pool, tenant_id).await?;
        let now = Utc::now().to_rfc3339();

        if let Some(mut branding) = existing {
            sqlx::query(
                r#"
                UPDATE tenant_branding
                SET company_name = ?, primary_color = ?, secondary_color = ?,
                    accent_color = ?, logo_url = ?, updated_at = ?
                WHERE id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(&input.company_name)
            .bind(&input.primary_color)
            .bind(&input.secondary_color)
            .bind(&input.accent_color)
            .bind(&input.logo_url)
            .bind(&now)
            .bind(&branding.id)
            .execute(pool)
            .await?;

            branding.company_name = input.company_name.clone();
            branding.primary_color = input.primary_color.clone();
            branding.secondary_color = input.secondary_color.clone();
            branding.accent_color = input.accent_color.clone();
            branding.logo_url = input.logo_url.clone();
            branding.updated_at = now;

            Ok(branding)
        } else {
            let new_id = Uuid::now_v7().to_string();

            sqlx::query(
                r#"
                INSERT INTO tenant_branding (
                    id, tenant_id, company_name, primary_color, secondary_color,
                    accent_color, logo_url, favicon_url, created_at, updated_at,
                    deleted_at, created_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, NULL, NULL)
                "#,
            )
            .bind(&new_id)
            .bind(tenant_id)
            .bind(&input.company_name)
            .bind(&input.primary_color)
            .bind(&input.secondary_color)
            .bind(&input.accent_color)
            .bind(&input.logo_url)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;

            Self::get_by_tenant_id(pool, tenant_id)
                .await?
                .ok_or_else(|| CoreError::NotFound(format!("Tenant branding for {tenant_id}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;

    #[tokio::test]
    async fn test_branding_repository_in_memory() {
        // Inicializa SQLite em memória com migrações
        let pool = init_db("sqlite::memory:")
            .await
            .expect("Falha ao inicializar banco de testes em memória");

        let default_tenant_id = "018f0000-0000-7000-8000-000000000001";

        // Verifica existência do branding pré-carregado na migração
        let initial = BrandingRepository::get_by_tenant_id(&pool, default_tenant_id)
            .await
            .expect("Erro ao buscar branding inicial")
            .expect("Branding padrão da migration deve existir");

        assert_eq!(initial.company_name, "EventRent Audiovisual");

        // Atualiza branding
        let input = UpdateBrandingInput {
            company_name: "Mega Som e Luz".to_string(),
            primary_color: "#10b981".to_string(),
            secondary_color: "#1e293b".to_string(),
            accent_color: "#f59e0b".to_string(),
            logo_url: Some("data:image/svg+xml;base64,...".to_string()),
        };

        let updated = BrandingRepository::save(&pool, default_tenant_id, &input)
            .await
            .expect("Erro ao salvar branding");

        assert_eq!(updated.company_name, "Mega Som e Luz");
        assert_eq!(updated.primary_color, "#10b981");
        assert_eq!(updated.secondary_color, "#1e293b");
        assert_eq!(updated.accent_color, "#f59e0b");
        assert_eq!(
            updated.logo_url,
            Some("data:image/svg+xml;base64,...".to_string())
        );

        // Confirma leitura após atualização
        let reloaded = BrandingRepository::get_by_tenant_id(&pool, default_tenant_id)
            .await
            .expect("Erro ao recarregar")
            .expect("Branding deve existir");

        assert_eq!(reloaded.company_name, "Mega Som e Luz");
        assert_eq!(reloaded.primary_color, "#10b981");
    }
}
