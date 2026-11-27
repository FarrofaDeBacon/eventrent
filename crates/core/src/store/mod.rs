use crate::domain::catalog::*;
use crate::domain::inventory::*;
use crate::domain::media::*;
use crate::domain::{
    AuditLogEntry, Customer, CustomerAddress, CustomerContact, CustomerFilter, CustomerPage,
    CustomerWithDetails, DocumentType, Role, RoleWithPermissions, SupplierCategory, Tenant,
    TenantBranding, TenantSetting, UpdateBrandingInput, UserRecord, UserWithRole, Venue,
    VenueContact, VenueFilter, VenuePage, VenueSpace, VenueWithDetails,
};
use crate::error::{CoreError, Result};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait TenantStore: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Tenant>>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;
    async fn create(&self, tenant: &Tenant) -> Result<Tenant>;
    async fn update_name(&self, id: &str, name: &str, updated_at: &str) -> Result<Tenant>;
}

#[async_trait]
pub trait BrandingStore: Send + Sync {
    async fn get_by_tenant_id(&self, tenant_id: &str) -> Result<Option<TenantBranding>>;
    async fn save(&self, tenant_id: &str, input: &UpdateBrandingInput) -> Result<TenantBranding>;
}

#[async_trait]
pub trait RoleStore: Send + Sync {
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<RoleWithPermissions>>;
    async fn find_by_id(
        &self,
        tenant_id: &str,
        role_id: &str,
    ) -> Result<Option<RoleWithPermissions>>;
    async fn find_by_slug(
        &self,
        tenant_id: &str,
        slug: &str,
    ) -> Result<Option<RoleWithPermissions>>;
    async fn create(&self, role: &Role, permissions: &[String]) -> Result<RoleWithPermissions>;
    async fn set_permissions(
        &self,
        tenant_id: &str,
        role_id: &str,
        permissions: &[String],
        updated_at: &str,
    ) -> Result<()>;
}

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn find_by_id(&self, tenant_id: &str, user_id: &str) -> Result<Option<UserRecord>>;
    async fn find_by_email(&self, tenant_id: &str, email: &str) -> Result<Option<UserRecord>>;
    async fn find_with_role(&self, tenant_id: &str, user_id: &str) -> Result<Option<UserWithRole>>;
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<UserWithRole>>;
    async fn count_active_by_role_slug(&self, tenant_id: &str, role_slug: &str) -> Result<i64>;
    async fn count_by_tenant(&self, tenant_id: &str) -> Result<i64>;
    async fn create(&self, user: &UserRecord) -> Result<UserWithRole>;
    async fn update(
        &self,
        tenant_id: &str,
        user_id: &str,
        name: &str,
        email: &str,
        role_id: &str,
        updated_at: &str,
    ) -> Result<UserWithRole>;
    async fn set_active(
        &self,
        tenant_id: &str,
        user_id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<UserWithRole>;
    async fn update_password(
        &self,
        tenant_id: &str,
        user_id: &str,
        password_hash: &str,
        updated_at: &str,
    ) -> Result<()>;
    async fn record_login_success(
        &self,
        tenant_id: &str,
        user_id: &str,
        login_at: &str,
    ) -> Result<()>;
    /// Incrementa atomicamente o contador de falhas de login em um único UPDATE e devolve o novo total.
    async fn increment_login_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        updated_at: &str,
    ) -> Result<i32>;
    /// Define o bloqueio temporário de login do usuário.
    async fn set_login_lockout(
        &self,
        tenant_id: &str,
        user_id: &str,
        locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()>;
    /// Incrementa atomicamente o contador de falhas da chave de recuperação em um único UPDATE e devolve o novo total.
    async fn increment_recovery_failure(
        &self,
        tenant_id: &str,
        user_id: &str,
        updated_at: &str,
    ) -> Result<i32>;
    /// Define o bloqueio temporário da chave de recuperação do usuário.
    async fn set_recovery_lockout(
        &self,
        tenant_id: &str,
        user_id: &str,
        locked_until: Option<&str>,
        updated_at: &str,
    ) -> Result<()>;
    /// Atualiza senha, zera contadores de bloqueio (login e recuperação) e grava novo hash da chave de recuperação.
    async fn update_password_and_recovery_key(
        &self,
        tenant_id: &str,
        user_id: &str,
        password_hash: &str,
        recovery_key_hash: &str,
        updated_at: &str,
    ) -> Result<()>;
    /// Atualiza exclusivamente a chave de recuperação, zerando falhas e bloqueios de recuperação.
    async fn update_recovery_key(
        &self,
        tenant_id: &str,
        user_id: &str,
        recovery_key_hash: &str,
        updated_at: &str,
    ) -> Result<()>;
    /// Exclusão lógica (soft delete) do usuário com incremento de auth_epoch para revogação de sessões.
    async fn soft_delete(&self, tenant_id: &str, user_id: &str, deleted_at: &str) -> Result<()>;
}

#[async_trait]
pub trait AuditStore: Send + Sync {
    async fn append(&self, entry: &AuditLogEntry) -> Result<()>;
    async fn list_by_tenant(&self, tenant_id: &str, limit: usize) -> Result<Vec<AuditLogEntry>>;
}

#[async_trait]
pub trait CustomerStore: Send + Sync {
    async fn find_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<CustomerWithDetails>>;
    async fn find_by_document(
        &self,
        tenant_id: &str,
        document_type: DocumentType,
        document_number: &str,
    ) -> Result<Option<Customer>>;
    async fn list(&self, tenant_id: &str, filter: &CustomerFilter) -> Result<CustomerPage>;
    async fn create(
        &self,
        customer: &Customer,
        contacts: &[CustomerContact],
        addresses: &[CustomerAddress],
        supplier_categories: &[SupplierCategory],
    ) -> Result<()>;
    async fn update(
        &self,
        customer: &Customer,
        contacts: &[CustomerContact],
        addresses: &[CustomerAddress],
        supplier_categories: &[SupplierCategory],
    ) -> Result<()>;
    async fn set_active(
        &self,
        tenant_id: &str,
        id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<()>;
    async fn soft_delete(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()>;
    async fn count_active(&self, tenant_id: &str) -> Result<u64>;
}

#[async_trait]
pub trait VenueStore: Send + Sync {
    async fn find_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<VenueWithDetails>>;
    async fn list(&self, tenant_id: &str, filter: &VenueFilter) -> Result<VenuePage>;
    async fn exists_by_name_and_city(
        &self,
        tenant_id: &str,
        name_normalized: &str,
        city_normalized: Option<&str>,
        exclude_id: Option<&str>,
    ) -> Result<bool>;
    async fn create(
        &self,
        venue: &Venue,
        spaces: &[VenueSpace],
        contacts: &[VenueContact],
    ) -> Result<()>;
    async fn update(
        &self,
        venue: &Venue,
        spaces: &[VenueSpace],
        contacts: &[VenueContact],
    ) -> Result<()>;
    async fn set_active(
        &self,
        tenant_id: &str,
        id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<()>;
    async fn soft_delete(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()>;
    async fn count_active(&self, tenant_id: &str) -> Result<u64>;
}

#[async_trait]
pub trait TenantSettingStore: Send + Sync {
    async fn get_many(&self, tenant_id: &str, keys: &[&str]) -> Result<Vec<TenantSetting>>;
    async fn upsert(&self, setting: &TenantSetting) -> Result<()>;
}

#[async_trait]
pub trait CatalogStore: Send + Sync {
    async fn list_brands(&self, tenant_id: &str, include_inactive: bool) -> Result<Vec<Brand>>;
    async fn find_brand_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<Brand>>;
    async fn find_brand_by_name_normalized(
        &self,
        tenant_id: &str,
        name_normalized: &str,
    ) -> Result<Option<Brand>>;
    async fn create_brand(&self, brand: &Brand) -> Result<()>;
    async fn update_brand(&self, brand: &Brand) -> Result<()>;
    async fn soft_delete_brand(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()>;
    async fn count_models_by_brand(&self, tenant_id: &str, brand_id: &str) -> Result<i64>;

    async fn list_categories(
        &self,
        tenant_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<EquipmentCategory>>;
    async fn find_category_by_id(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<EquipmentCategory>>;
    async fn find_category_by_parent_and_name(
        &self,
        tenant_id: &str,
        parent_id: Option<&str>,
        name_normalized: &str,
    ) -> Result<Option<EquipmentCategory>>;
    async fn create_category(&self, category: &EquipmentCategory) -> Result<()>;
    async fn update_category(&self, category: &EquipmentCategory) -> Result<()>;
    async fn soft_delete_category(&self, tenant_id: &str, id: &str, deleted_at: &str)
        -> Result<()>;
    async fn count_child_categories(&self, tenant_id: &str, category_id: &str) -> Result<i64>;
    async fn count_models_by_category(&self, tenant_id: &str, category_id: &str) -> Result<i64>;
    async fn count_models_per_category(&self, tenant_id: &str) -> Result<Vec<(String, i64)>>;

    async fn list_all_attribute_definitions(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<AttributeDefinition>>;
    async fn list_attribute_definitions_by_category(
        &self,
        tenant_id: &str,
        category_id: &str,
    ) -> Result<Vec<AttributeDefinition>>;
    async fn find_attribute_definition_by_id(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> Result<Option<AttributeDefinition>>;
    async fn create_attribute_definition(&self, def: &AttributeDefinition) -> Result<()>;
    async fn update_attribute_definition(&self, def: &AttributeDefinition) -> Result<()>;
    async fn soft_delete_attribute_definition(
        &self,
        tenant_id: &str,
        id: &str,
        deleted_at: &str,
    ) -> Result<()>;
    async fn count_models_using_attribute_definition(
        &self,
        tenant_id: &str,
        definition_id: &str,
    ) -> Result<i64>;
    async fn list_distinct_text_values_for_attribute_definition(
        &self,
        tenant_id: &str,
        definition_id: &str,
    ) -> Result<Vec<String>>;

    async fn list_models(
        &self,
        tenant_id: &str,
        filter: &CatalogListFilter,
        descendant_category_ids: Option<&[String]>,
    ) -> Result<CatalogListResult>;
    async fn find_model_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<EquipmentModel>>;
    async fn list_model_attribute_values(
        &self,
        tenant_id: &str,
        model_id: &str,
    ) -> Result<Vec<EquipmentModelAttributeValue>>;
    async fn create_model(
        &self,
        model: &EquipmentModel,
        values: &[EquipmentModelAttributeValue],
    ) -> Result<()>;
    async fn update_model(
        &self,
        model: &EquipmentModel,
        values: &[EquipmentModelAttributeValue],
    ) -> Result<()>;
    async fn set_model_active(
        &self,
        tenant_id: &str,
        id: &str,
        active: bool,
        updated_at: &str,
    ) -> Result<()>;
    async fn soft_delete_model(&self, tenant_id: &str, id: &str, deleted_at: &str) -> Result<()>;
    async fn count_active_models(&self, tenant_id: &str) -> Result<u64>;
    async fn list_models_by_brand(
        &self,
        tenant_id: &str,
        brand_id: &str,
    ) -> Result<Vec<EquipmentModel>>;
    async fn list_models_by_category(
        &self,
        tenant_id: &str,
        category_id: &str,
    ) -> Result<Vec<EquipmentModel>>;
    async fn update_model_search_text(
        &self,
        tenant_id: &str,
        model_id: &str,
        search_text: &str,
    ) -> Result<()>;
    async fn find_model_by_sku_normalized(
        &self,
        tenant_id: &str,
        sku_normalized: &str,
    ) -> Result<Option<EquipmentModel>>;
    async fn find_model_by_brand_and_code_normalized(
        &self,
        tenant_id: &str,
        brand_id: &str,
        model_code_normalized: &str,
    ) -> Result<Option<EquipmentModel>>;

    async fn find_media_blob_by_id(
        &self,
        _tenant_id: &str,
        _blob_id: &str,
    ) -> Result<Option<MediaBlob>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn find_media_blob_by_sha256_and_mime(
        &self,
        _tenant_id: &str,
        _sha256: &str,
        _mime_type: MediaMimeType,
    ) -> Result<Option<MediaBlob>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn create_media_blob(&self, _blob: &MediaBlob) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn find_media_asset_by_id(
        &self,
        _tenant_id: &str,
        _asset_id: &str,
    ) -> Result<Option<MediaAsset>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn create_media_asset(&self, _asset: &MediaAsset) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn update_media_asset(&self, _asset: &MediaAsset) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn list_model_media_views(
        &self,
        _tenant_id: &str,
        _model_id: &str,
    ) -> Result<Vec<EquipmentModelMediaView>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn find_model_media_link_by_id(
        &self,
        _tenant_id: &str,
        _link_id: &str,
    ) -> Result<Option<EquipmentModelMedia>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn find_model_media_view_by_link_id(
        &self,
        _tenant_id: &str,
        _link_id: &str,
    ) -> Result<Option<EquipmentModelMediaView>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn count_model_media_by_kind(
        &self,
        _tenant_id: &str,
        _model_id: &str,
        _kind: MediaKind,
    ) -> Result<usize> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn create_model_media_link(&self, _link: &EquipmentModelMedia) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn update_model_media_link(&self, _link: &EquipmentModelMedia) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn clear_primary_model_photo(
        &self,
        _tenant_id: &str,
        _model_id: &str,
        _updated_at: &str,
        _updated_by: Option<&str>,
    ) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn soft_delete_model_media_link(
        &self,
        _tenant_id: &str,
        _link_id: &str,
        _deleted_at: &str,
        _updated_by: Option<&str>,
    ) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn purge_soft_deleted_model_media_links(&self, _tenant_id: &str) -> Result<usize> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn list_unreferenced_media_assets(&self, _tenant_id: &str) -> Result<Vec<MediaAsset>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn hard_delete_media_asset(&self, _tenant_id: &str, _asset_id: &str) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn list_unreferenced_media_blobs(
        &self,
        _tenant_id: &str,
        _created_before: &str,
    ) -> Result<Vec<MediaBlob>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn list_active_media_blob_shas(
        &self,
        _tenant_id: &str,
    ) -> Result<std::collections::HashSet<String>> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
    async fn hard_delete_media_blob(&self, _tenant_id: &str, _blob_id: &str) -> Result<()> {
        Err(CoreError::Internal("Não implementado".to_string()))
    }
}

/// Trait de armazenamento físico de blobs de mídia (CAS por SHA-256 e tenant_id — ADR-0013).
#[async_trait]
pub trait MediaStore: Send + Sync {
    async fn put_blob(&self, tenant_id: &str, sha256: &str, bytes: &[u8]) -> Result<String>;
    async fn get_blob(&self, tenant_id: &str, sha256: &str) -> Result<Vec<u8>>;
    async fn delete_blob(&self, tenant_id: &str, sha256: &str) -> Result<()>;
    async fn exists_blob(&self, tenant_id: &str, sha256: &str) -> Result<bool>;
    /// Reconcilia arquivos físicos com o banco: remove arquivos órfãos sem linha correspondente há mais de min_age (R7.3).
    async fn reconcile_unreferenced_blobs(
        &self,
        _tenant_id: &str,
        _valid_shas: &std::collections::HashSet<String>,
        _min_age: std::time::Duration,
    ) -> Result<usize> {
        Ok(0)
    }
}

#[async_trait]
pub trait InventoryStore: Send + Sync {
    // Depósitos (Warehouses)
    async fn create_warehouse(&self, warehouse: &Warehouse) -> Result<()>;
    async fn update_warehouse(&self, warehouse: &Warehouse) -> Result<()>;
    async fn find_warehouse_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<Warehouse>>;
    async fn find_warehouse_by_code(
        &self,
        tenant_id: &str,
        code: &str,
    ) -> Result<Option<Warehouse>>;
    async fn get_default_warehouse(&self, tenant_id: &str) -> Result<Option<Warehouse>>;
    async fn list_warehouses(
        &self,
        tenant_id: &str,
        include_inactive: bool,
    ) -> Result<Vec<Warehouse>>;
    async fn set_default_warehouse(&self, tenant_id: &str, id: &str) -> Result<()>;
    async fn delete_warehouse(&self, tenant_id: &str, id: &str) -> Result<()>;

    // Unidades Físicas (Equipment Units)
    async fn create_unit(&self, unit: &EquipmentUnit) -> Result<()>;
    async fn update_unit(&self, unit: &EquipmentUnit) -> Result<()>;
    async fn find_unit_by_id(&self, tenant_id: &str, id: &str) -> Result<Option<EquipmentUnit>>;
    async fn find_unit_by_patrimony(
        &self,
        tenant_id: &str,
        tag: &str,
    ) -> Result<Option<EquipmentUnit>>;
    async fn list_units(
        &self,
        tenant_id: &str,
        filter: &EquipmentUnitFilter,
    ) -> Result<Vec<EquipmentUnit>>;
    async fn list_unit_details(
        &self,
        tenant_id: &str,
        filter: &EquipmentUnitFilter,
    ) -> Result<EquipmentUnitListPage>;
    async fn next_patrimony_tag(&self, tenant_id: &str) -> Result<String>;
    async fn count_unit_movements(&self, tenant_id: &str, unit_id: &str) -> Result<i64>;
    async fn delete_unit(&self, tenant_id: &str, id: &str) -> Result<()>;

    // Estoque & Saldos (Stock & Balances)
    async fn get_stock_balance(
        &self,
        tenant_id: &str,
        warehouse_id: &str,
        equipment_model_id: &str,
    ) -> Result<Option<StockBalance>>;
    async fn list_stock_balances_by_model(
        &self,
        tenant_id: &str,
        equipment_model_id: &str,
    ) -> Result<Vec<StockBalance>>;
    async fn list_stock_balances_by_warehouse(
        &self,
        tenant_id: &str,
        warehouse_id: &str,
    ) -> Result<Vec<StockBalance>>;
    async fn record_stock_movement(&self, movement: &StockMovement) -> Result<()>;
    async fn list_stock_movements(
        &self,
        tenant_id: &str,
        filter: &StockMovementFilter,
    ) -> Result<Vec<StockMovement>>;
}

#[derive(Default)]
pub struct InMemoryMediaStore {
    blobs: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

impl InMemoryMediaStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MediaStore for InMemoryMediaStore {
    async fn put_blob(&self, tenant_id: &str, sha256: &str, bytes: &[u8]) -> Result<String> {
        let clean_sha = validate_sha256_hex(sha256)?;
        let actual_sha = compute_sha256_hex(bytes);
        if actual_sha != clean_sha {
            return Err(CoreError::Validation(
                "Integridade de mídia inválida: hash SHA-256 não confere com o conteúdo."
                    .to_string(),
            ));
        }
        let key = build_media_storage_key(tenant_id, &clean_sha)?;
        let mut guard = self
            .blobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.insert(key.clone(), bytes.to_vec());
        Ok(key)
    }

    async fn get_blob(&self, tenant_id: &str, sha256: &str) -> Result<Vec<u8>> {
        let key = build_media_storage_key(tenant_id, sha256)?;
        let guard = self
            .blobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.get(&key).cloned().ok_or_else(|| {
            CoreError::NotFound(format!(
                "Arquivo de mídia '{}' não encontrado no armazenamento.",
                key
            ))
        })
    }

    async fn delete_blob(&self, tenant_id: &str, sha256: &str) -> Result<()> {
        let key = build_media_storage_key(tenant_id, sha256)?;
        let mut guard = self
            .blobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.remove(&key);
        Ok(())
    }

    async fn exists_blob(&self, tenant_id: &str, sha256: &str) -> Result<bool> {
        let key = build_media_storage_key(tenant_id, sha256)?;
        let guard = self
            .blobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(guard.contains_key(&key))
    }

    async fn reconcile_unreferenced_blobs(
        &self,
        tenant_id: &str,
        valid_shas: &std::collections::HashSet<String>,
        _min_age: std::time::Duration,
    ) -> Result<usize> {
        let prefix = format!("{}/", tenant_id);
        let mut guard = self
            .blobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut to_remove = Vec::new();
        for key in guard.keys() {
            if key.starts_with(&prefix) {
                if let Some(sha) = key.rsplit('/').next() {
                    if !valid_shas.contains(sha) {
                        to_remove.push(key.clone());
                    }
                }
            }
        }
        let count = to_remove.len();
        for key in &to_remove {
            guard.remove(key);
        }
        Ok(count)
    }
}

#[derive(Clone)]
pub struct StoreBundle {
    pub tenants: Arc<dyn TenantStore>,
    pub branding: Arc<dyn BrandingStore>,
    pub roles: Arc<dyn RoleStore>,
    pub users: Arc<dyn UserStore>,
    pub audit: Arc<dyn AuditStore>,
    pub customers: Arc<dyn CustomerStore>,
    pub venues: Arc<dyn VenueStore>,
    pub settings: Arc<dyn TenantSettingStore>,
    pub catalog: Arc<dyn CatalogStore>,
    pub inventory: Arc<dyn InventoryStore>,
    pub tx_manager: Option<Arc<dyn TransactionManager>>,
}

impl StoreBundle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenants: Arc<dyn TenantStore>,
        branding: Arc<dyn BrandingStore>,
        roles: Arc<dyn RoleStore>,
        users: Arc<dyn UserStore>,
        audit: Arc<dyn AuditStore>,
        customers: Arc<dyn CustomerStore>,
        venues: Arc<dyn VenueStore>,
        settings: Arc<dyn TenantSettingStore>,
        catalog: Arc<dyn CatalogStore>,
        inventory: Arc<dyn InventoryStore>,
        tx_manager: Option<Arc<dyn TransactionManager>>,
    ) -> Result<Self> {
        let Some(tx) = tx_manager else {
            return Err(CoreError::Configuration(
                "TransactionManager (tx_manager) é obrigatório no StoreBundle para garantir atomicidade transacional.".to_string(),
            ));
        };
        Ok(Self {
            tenants,
            branding,
            roles,
            users,
            audit,
            customers,
            venues,
            settings,
            catalog,
            inventory,
            tx_manager: Some(tx),
        })
    }
}

#[async_trait]
pub trait TransactionManager: Send + Sync {
    async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>>;
}

#[async_trait]
pub trait TxStoreBundle: Send + Sync {
    fn tenants(&self) -> &dyn TenantStore;
    fn branding(&self) -> &dyn BrandingStore;
    fn roles(&self) -> &dyn RoleStore;
    fn users(&self) -> &dyn UserStore;
    fn audit(&self) -> &dyn AuditStore;
    fn customers(&self) -> &dyn CustomerStore;
    fn venues(&self) -> &dyn VenueStore;
    fn settings(&self) -> &dyn TenantSettingStore;
    fn catalog(&self) -> &dyn CatalogStore;
    fn inventory(&self) -> &dyn InventoryStore;

    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TransactionPolicy {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
}

impl Default for TransactionPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 25,
        }
    }
}

use std::future::Future;
use std::pin::Pin;

pub type TxBoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Executa um closure assíncrono dentro de uma transação atômica gerenciada por `TransactionManager`.
///
/// Garantias:
/// 1. Commit automático quando o closure retornar `Ok(T)`.
/// 2. Rollback automático quando o closure retornar `Err(E)` ou ocorrer pânico (RAII/Drop).
/// 3. Nova tentativa automática com backoff exponencial quando o erro for `CoreError::TransientConflict`
///    (ex.: SQLITE_BUSY / lock contention sob concorrência).
pub async fn run_in_tx<F, T>(
    manager: &dyn TransactionManager,
    policy: &TransactionPolicy,
    f: F,
) -> Result<T>
where
    F: for<'a> Fn(&'a dyn TxStoreBundle) -> TxBoxFuture<'a, T>,
{
    let mut attempt = 0;
    loop {
        let tx = match manager.begin_transaction().await {
            Ok(tx) => tx,
            Err(err) => {
                let is_transient = matches!(err, CoreError::TransientConflict(_));
                if is_transient && attempt < policy.max_retries {
                    attempt += 1;
                    let backoff = policy.initial_backoff_ms * (1 << (attempt - 1));
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                    continue;
                }
                return Err(err);
            }
        };

        match f(tx.as_ref()).await {
            Ok(res) => match tx.commit().await {
                Ok(()) => return Ok(res),
                Err(commit_err) => {
                    let is_transient = matches!(commit_err, CoreError::TransientConflict(_));
                    if is_transient && attempt < policy.max_retries {
                        attempt += 1;
                        let backoff = policy.initial_backoff_ms * (1 << (attempt - 1));
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                        continue;
                    }
                    return Err(commit_err);
                }
            },
            Err(err) => {
                let is_transient = matches!(err, CoreError::TransientConflict(_));
                let _ = tx.rollback().await;

                if is_transient && attempt < policy.max_retries {
                    attempt += 1;
                    let backoff = policy.initial_backoff_ms * (1 << (attempt - 1));
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                    continue;
                }

                return Err(err);
            }
        }
    }
}
