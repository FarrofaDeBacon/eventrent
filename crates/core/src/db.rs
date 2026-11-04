use crate::error::Result;
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Row, Sqlite};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

pub type DbPool = Pool<Sqlite>;

/// Inicializa o banco de dados SQLite em arquivo usando `SqliteConnectOptions::new().filename(path)`,
/// garantindo compatibilidade com caminhos contendo espaços e acentuação no Windows, macOS e Linux.
pub async fn init_db(db_path: &Path) -> Result<DbPool> {
    let connection_options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

/// Inicializa um banco SQLite em memória isolado para testes automatizados.
pub async fn init_memory_db() -> Result<DbPool> {
    let connection_options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(connection_options)
        .await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(crate::error::CoreError::from)?;
    Ok(())
}

/// Garante de forma idempotente a existência do tenant padrão (`slug = 'default'`)
/// e de seu respectivo registro em `tenant_branding` para operação desktop,
/// utilizando UUID v7 gerados dinamicamente e timestamps reais em UTC (ISO-8601).
/// Retorna o `id` (UUID v7) do tenant padrão.
pub async fn ensure_default_tenant(pool: &DbPool) -> Result<String> {
    let mut tx = pool.begin().await?;
    let now = Utc::now().to_rfc3339();

    let existing_tenant = sqlx::query(
        r#"
        SELECT id
        FROM tenant
        WHERE slug = 'default' AND deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let tenant_id = if let Some(row) = existing_tenant {
        row.get::<String, _>("id")
    } else {
        let new_tenant_id = Uuid::now_v7().to_string();
        sqlx::query(
            r#"
            INSERT INTO tenant (
                id, tenant_id, name, slug, document_number,
                created_at, updated_at, deleted_at, created_by
            ) VALUES (?, ?, 'EventRent Padrão', 'default', NULL, ?, ?, NULL, NULL)
            "#,
        )
        .bind(&new_tenant_id)
        .bind(&new_tenant_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        new_tenant_id
    };

    let existing_branding = sqlx::query(
        r#"
        SELECT id
        FROM tenant_branding
        WHERE tenant_id = ? AND deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(&tenant_id)
    .fetch_optional(&mut *tx)
    .await?;

    if existing_branding.is_none() {
        let new_branding_id = Uuid::now_v7().to_string();
        sqlx::query(
            r#"
            INSERT INTO tenant_branding (
                id, tenant_id, company_name, primary_color, secondary_color,
                accent_color, logo_url, favicon_url, created_at, updated_at,
                deleted_at, created_by
            ) VALUES (
                ?, ?, 'EventRent Audiovisual', '#2563eb', '#475569',
                '#7c3aed', NULL, NULL, ?, ?, NULL, NULL
            )
            "#,
        )
        .bind(&new_branding_id)
        .bind(&tenant_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(tenant_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::BrandingRepository;

    #[tokio::test]
    async fn test_init_db_with_spaces_and_accents_in_path() {
        let temp_base = std::env::temp_dir().join(format!(
            "eventrent_test_{}_João Silva & Locação São Paulo",
            Uuid::now_v7()
        ));
        std::fs::create_dir_all(&temp_base).expect("Falha ao criar pasta temporária com acentos");

        let db_path = temp_base.join("dados de locação áudio & vídeo.db");

        let pool = init_db(&db_path)
            .await
            .expect("Falha ao inicializar banco em caminho com espaços e acentos");

        let tenant_id_first = ensure_default_tenant(&pool)
            .await
            .expect("Falha ao garantir tenant padrão");
        let tenant_id_second = ensure_default_tenant(&pool)
            .await
            .expect("Segunda chamada idempotente a ensure_default_tenant falhou");

        assert_eq!(tenant_id_first, tenant_id_second);
        assert!(Uuid::parse_str(&tenant_id_first).is_ok());

        let branding = BrandingRepository::get_by_tenant_id(&pool, &tenant_id_first)
            .await
            .expect("Falha ao ler branding no caminho com acentos")
            .expect("Branding padrão deve existir após ensure_default_tenant");

        assert_eq!(branding.company_name, "EventRent Audiovisual");
        assert!(db_path.exists());

        pool.close().await;
        let _ = std::fs::remove_dir_all(&temp_base);
    }
}
