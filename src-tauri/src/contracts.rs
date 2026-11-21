use crate::guards::COMMAND_REGISTRY;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractFixture {
    pub command: String,
    pub is_public: bool,
    pub has_return_value: bool,
    pub sample_output: serde_json::Value,
    pub sample_errors: BTreeMap<String, String>,
}

fn standard_errors_for(
    is_public: bool,
    has_lookup: bool,
    has_mutation: bool,
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if !is_public {
        map.insert(
            "Unauthorized".to_string(),
            "Não autenticado: Sessão ausente, inválida ou expirada.".to_string(),
        );
        map.insert(
            "Forbidden".to_string(),
            "Permissão negada: requer 'recurso:acao'".to_string(),
        );
    }
    if has_mutation {
        map.insert(
            "Validation".to_string(),
            "Erro de validação: Dados de entrada inválidos para a operação.".to_string(),
        );
        map.insert(
            "Conflict".to_string(),
            "Conflito de registro: Já existe um registro com os dados informados.".to_string(),
        );
    }
    if has_lookup {
        map.insert(
            "NotFound".to_string(),
            "Registro não encontrado: O identificador solicitado não existe.".to_string(),
        );
    }
    map
}

pub fn build_fixture_for_command(cmd_name: &str) -> Option<ContractFixture> {
    let def = COMMAND_REGISTRY.iter().find(|c| c.name == cmd_name)?;
    let is_public = def.is_public;

    let (has_return_value, sample_output, sample_errors) = match cmd_name {
        "get_app_status" => (
            true,
            json!({
                "config": {
                    "mode": "local",
                    "setup_completed": true,
                    "inactivity_timeout_minutes": 30
                },
                "default_tenant_id": "018f0000-0000-7000-8000-000000000001",
                "has_owner": true,
                "session": null,
                "branding": {
                    "company_name": "EventRent Audiovisual",
                    "trading_name": "EventRent",
                    "primary_color": "#E11D48",
                    "secondary_color": "#1E293B",
                    "accent_color": "#F59E0B",
                    "logo_url": null,
                    "logo_dark_url": null,
                    "icon_url": null
                }
            }),
            BTreeMap::new(),
        ),
        "complete_initial_setup" => {
            let mut errs = BTreeMap::new();
            errs.insert(
                "Validation".to_string(),
                "Erro de validação: Senha deve ter pelo menos 8 caracteres.".to_string(),
            );
            errs.insert(
                "Conflict".to_string(),
                "Conflito de registro: Sistema já configurado.".to_string(),
            );
            (
                true,
                json!({
                    "tenant_id": "018f0000-0000-7000-8000-000000000001",
                    "recovery_key": "ER-ABCD-EFGH-1234-5678",
                    "session": sample_session_json(),
                    "branding": sample_branding_json()
                }),
                errs,
            )
        }
        "get_public_branding" => (
            true,
            json!({
                "company_name": "EventRent Audiovisual",
                "primary_color": "#E11D48",
                "secondary_color": "#1E293B",
                "accent_color": "#F59E0B",
                "logo_url": null,
                "logo_dark_url": null,
                "icon_url": null
            }),
            BTreeMap::new(),
        ),
        "login" => {
            let mut errs = BTreeMap::new();
            errs.insert(
                "InvalidCredentials".to_string(),
                "E-mail ou senha inválidos.".to_string(),
            );
            errs.insert(
                "AccountLocked".to_string(),
                "Conta temporariamente bloqueada até 2026-11-21T12:00:00Z".to_string(),
            );
            errs.insert(
                "Validation".to_string(),
                "Erro de validação: O e-mail deve conter apenas caracteres ASCII.".to_string(),
            );
            (true, sample_session_json(), errs)
        }
        "logout" => (false, serde_json::Value::Null, BTreeMap::new()),
        "get_current_session" => (true, sample_session_json(), BTreeMap::new()),
        "recover_password" => {
            let mut errs = BTreeMap::new();
            errs.insert(
                "InvalidCredentials".to_string(),
                "E-mail ou chave de recuperação inválidos.".to_string(),
            );
            errs.insert(
                "AccountLocked".to_string(),
                "Conta temporariamente bloqueada até 2026-11-21T12:00:00Z".to_string(),
            );
            errs.insert(
                "Validation".to_string(),
                "Erro de validação: Nova senha deve ter no mínimo 8 caracteres.".to_string(),
            );
            (
                true,
                json!({
                    "new_recovery_key": "ER-ROTATED-KEY-2026-0001"
                }),
                errs,
            )
        }
        "regenerate_recovery_key" => {
            let mut errs = standard_errors_for(false, false, true);
            errs.insert(
                "InvalidCredentials".to_string(),
                "Senha atual incorreta.".to_string(),
            );
            errs.insert(
                "AccountLocked".to_string(),
                "Conta temporariamente bloqueada até 2026-11-21T12:00:00Z".to_string(),
            );
            (
                true,
                json!({
                    "new_recovery_key": "ER-REGEN-KEY-2026-0002"
                }),
                errs,
            )
        }
        "get_branding" | "save_branding" => (
            true,
            sample_branding_json(),
            standard_errors_for(false, false, cmd_name == "save_branding"),
        ),
        "list_users" => (
            true,
            json!([sample_user_json()]),
            standard_errors_for(false, false, false),
        ),
        "create_user" | "update_user" | "set_user_active" => (
            true,
            sample_user_json(),
            standard_errors_for(false, true, true),
        ),
        "reset_user_password" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "list_roles" => (
            true,
            json!([{
                "id": "018f0000-0000-7000-8000-000000000031",
                "name": "Proprietário",
                "slug": "proprietario",
                "description": "Acesso total",
                "permissions": ["all"],
                "is_system": true
            }]),
            standard_errors_for(false, false, false),
        ),
        "list_customers" => (
            true,
            json!({
                "items": [{
                    "id": "018f0000-0000-7000-8000-000000000040",
                    "tenant_id": "018f0000-0000-7000-8000-000000000001",
                    "person_type": "legal",
                    "name": "Showtime Produções LTDA",
                    "trade_name": "Showtime",
                    "document_number": "12345678000195",
                    "document_type": "cnpj",
                    "primary_phone": "+5511987654321",
                    "primary_email": "contato@showtime.com.br",
                    "primary_city": "São Paulo",
                    "primary_state": "SP",
                    "roles": ["customer"],
                    "active": true,
                    "created_at": "2026-11-21T10:00:00Z",
                    "updated_at": "2026-11-21T10:00:00Z"
                }],
                "total": 1,
                "next_cursor": null
            }),
            standard_errors_for(false, false, false),
        ),
        "get_customer"
        | "create_customer"
        | "update_customer"
        | "set_customer_active"
        | "add_customer_role" => (
            true,
            sample_customer_with_details_json(),
            standard_errors_for(false, true, cmd_name != "get_customer"),
        ),
        "delete_customer" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "list_venues" => (
            true,
            json!({
                "items": [{
                    "id": "018f0000-0000-7000-8000-000000000050",
                    "tenant_id": "018f0000-0000-7000-8000-000000000001",
                    "name": "Espaço das Américas",
                    "city": "São Paulo",
                    "state": "SP",
                    "spaces_count": 1,
                    "primary_contact_name": "João Gerente",
                    "primary_contact_phone": "+5511987654321",
                    "active": true,
                    "created_at": "2026-11-21T10:00:00Z",
                    "updated_at": "2026-11-21T10:00:00Z"
                }],
                "total": 1,
                "next_cursor": null
            }),
            standard_errors_for(false, false, false),
        ),
        "get_venue" | "create_venue" | "update_venue" | "set_venue_active" => (
            true,
            sample_venue_with_details_json(),
            standard_errors_for(false, true, cmd_name != "get_venue"),
        ),
        "delete_venue" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "check_venue_duplicate_name_city" => {
            (true, json!(false), standard_errors_for(false, false, false))
        }
        "get_allocation_settings" | "update_allocation_settings" => (
            true,
            json!({
                "id": "018f0000-0000-7000-8000-000000000020",
                "tenant_id": "018f0000-0000-7000-8000-000000000001",
                "internal_transfer_minutes": 30,
                "external_transfer_minutes": 60,
                "changeover_minutes": 15,
                "updated_at": "2026-11-21T10:00:00Z",
                "updated_by": null
            }),
            standard_errors_for(false, false, cmd_name == "update_allocation_settings"),
        ),
        "list_brands" => (
            true,
            json!([sample_brand_json()]),
            standard_errors_for(false, false, false),
        ),
        "create_brand" | "update_brand" => (
            true,
            sample_brand_json(),
            standard_errors_for(false, true, true),
        ),
        "delete_brand" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "list_categories_tree" => (
            true,
            json!([{
                "category": sample_category_json(),
                "children": []
            }]),
            standard_errors_for(false, false, false),
        ),
        "get_category_effective_attributes" => (
            true,
            json!([{
                "definition": sample_attribute_def_json(),
                "origin_category_id": "018f0000-0000-7000-8000-000000000070",
                "origin_category_name": "Áudio"
            }]),
            standard_errors_for(false, true, false),
        ),
        "create_category" | "update_category" => (
            true,
            sample_category_json(),
            standard_errors_for(false, true, true),
        ),
        "delete_category" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "create_attribute_definition" | "update_attribute_definition" => (
            true,
            sample_attribute_def_json(),
            standard_errors_for(false, true, true),
        ),
        "delete_attribute_definition" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "list_equipment_models" => (
            true,
            json!({
                "items": [{
                    "model": sample_model_json(),
                    "category_name": "Áudio",
                    "brand_name": "Pioneer DJ",
                    "primary_thumb_blob_id": null,
                    "primary_thumb_mime_type": null
                }],
                "next_cursor": null,
                "total_count": 1
            }),
            standard_errors_for(false, false, false),
        ),
        "get_equipment_model" | "create_equipment_model" | "update_equipment_model" => (
            true,
            json!({
                "model": sample_model_json(),
                "category_name": "Áudio",
                "brand_name": "Pioneer DJ",
                "effective_attributes": [],
                "attribute_values": []
            }),
            standard_errors_for(false, true, cmd_name != "get_equipment_model"),
        ),
        "set_equipment_model_active" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "delete_equipment_model" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "load_suggested_catalog_categories" => (
            true,
            json!({
                "categories_created": 7,
                "attributes_created": 21
            }),
            standard_errors_for(false, false, true),
        ),
        "list_model_media" => (
            true,
            json!([sample_media_view_json()]),
            standard_errors_for(false, true, false),
        ),
        "get_media_blob_bytes" => (
            true,
            json!([137, 80, 78, 71]),
            standard_errors_for(false, true, false),
        ),
        "attach_model_photo"
        | "attach_model_document"
        | "update_model_media_metadata"
        | "set_primary_model_photo" => (
            true,
            sample_media_view_json(),
            standard_errors_for(false, true, true),
        ),
        "reorder_model_media" => (
            true,
            json!([sample_media_view_json()]),
            standard_errors_for(false, true, true),
        ),
        "remove_model_media" => (
            false,
            serde_json::Value::Null,
            standard_errors_for(false, true, true),
        ),
        "collect_unreferenced_media" => (
            true,
            json!({
                "deleted_assets": 0,
                "deleted_blobs": 0,
                "deleted_files": 0
            }),
            standard_errors_for(false, false, true),
        ),
        "open_model_document_external" => (
            true,
            json!("C:\\Temp\\eventrent_media\\manual.pdf"),
            standard_errors_for(false, true, false),
        ),
        "save_model_document_as" => (
            true,
            json!("C:\\Users\\User\\Documents\\manual.pdf"),
            standard_errors_for(false, true, false),
        ),
        _ => return None,
    };

    Some(ContractFixture {
        command: cmd_name.to_string(),
        is_public,
        has_return_value,
        sample_output,
        sample_errors,
    })
}

fn sample_session_json() -> serde_json::Value {
    json!({
        "token": "sample-token-jwt-login",
        "tenant_id": "018f0000-0000-7000-8000-000000000001",
        "user_id": "018f0000-0000-7000-8000-000000000002",
        "name": "Operador Audiovisual",
        "email": "operador@eventrent.com.br",
        "role_slug": "proprietario",
        "role_name": "Proprietário",
        "permissions": ["all"],
        "mode": "local",
        "inactivity_timeout_minutes": 30,
        "last_activity_at": "2026-11-21T10:00:00Z"
    })
}

fn sample_branding_json() -> serde_json::Value {
    json!({
        "company_name": "EventRent Audiovisual",
        "trading_name": "EventRent",
        "primary_color": "#E11D48",
        "secondary_color": "#1E293B",
        "accent_color": "#F59E0B",
        "logo_url": null,
        "logo_dark_url": null,
        "icon_url": null,
        "updated_at": "2026-11-21T10:00:00Z"
    })
}

fn sample_user_json() -> serde_json::Value {
    json!({
        "id": "018f0000-0000-7000-8000-000000000030",
        "tenant_id": "018f0000-0000-7000-8000-000000000001",
        "name": "Carlos Técnico",
        "email": "carlos@eventrent.com.br",
        "role_id": "018f0000-0000-7000-8000-000000000031",
        "role_name": "Técnico",
        "role_slug": "tecnico",
        "permissions": ["catalog:read"],
        "active": true,
        "last_login_at": "2026-11-21T10:00:00Z",
        "failed_attempts": 0,
        "locked_until": null,
        "created_at": "2026-11-21T10:00:00Z",
        "updated_at": "2026-11-21T10:00:00Z",
        "deleted_at": null,
        "created_by": null
    })
}

fn sample_customer_with_details_json() -> serde_json::Value {
    json!({
        "customer": {
            "id": "018f0000-0000-7000-8000-000000000040",
            "tenant_id": "018f0000-0000-7000-8000-000000000001",
            "person_type": "legal",
            "name": "Showtime Produções LTDA",
            "trade_name": "Showtime",
            "document_number": "12345678000195",
            "document_type": "cnpj",
            "state_registration": null,
            "municipal_registration": null,
            "credit_limit_cents": 1000000,
            "notes": null,
            "active": true,
            "created_at": "2026-11-21T10:00:00Z",
            "updated_at": "2026-11-21T10:00:00Z",
            "deleted_at": null,
            "created_by": null,
            "updated_by": null,
            "search_text": "showtime producoes ltda"
        },
        "addresses": [],
        "contacts": [],
        "roles": ["customer"]
    })
}

fn sample_venue_with_details_json() -> serde_json::Value {
    json!({
        "venue": {
            "id": "018f0000-0000-7000-8000-000000000050",
            "tenant_id": "018f0000-0000-7000-8000-000000000001",
            "name": "Espaço das Américas",
            "name_sort": "espaco das americas",
            "search_text": "espaco das americas sao paulo sp",
            "street": "Rua Tagipuru",
            "number": "795",
            "complement": null,
            "neighborhood": "Barra Funda",
            "city": "São Paulo",
            "state": "SP",
            "postal_code": "01156000",
            "country": "BR",
            "access_conditions": null,
            "power_voltage": null,
            "power_distribution_distance_meters": null,
            "main_grounding": null,
            "generator_space": null,
            "water_point": null,
            "dock_levelers": null,
            "door_width_meters": null,
            "door_height_meters": null,
            "internal_transfer_minutes": 30,
            "notes": null,
            "active": true,
            "created_at": "2026-11-21T10:00:00Z",
            "updated_at": "2026-11-21T10:00:00Z",
            "deleted_at": null,
            "created_by": null,
            "updated_by": null
        },
        "spaces": [],
        "contacts": []
    })
}

fn sample_brand_json() -> serde_json::Value {
    json!({
        "id": "018f0000-0000-7000-8000-000000000060",
        "tenant_id": "018f0000-0000-7000-8000-000000000001",
        "name": "Pioneer DJ",
        "active": true,
        "created_at": "2026-11-21T10:00:00Z",
        "updated_at": "2026-11-21T10:00:00Z"
    })
}

fn sample_category_json() -> serde_json::Value {
    json!({
        "id": "018f0000-0000-7000-8000-000000000070",
        "tenant_id": "018f0000-0000-7000-8000-000000000001",
        "parent_id": null,
        "name": "Áudio",
        "full_path": "Áudio",
        "level": 1,
        "active": true,
        "created_at": "2026-11-21T10:00:00Z",
        "updated_at": "2026-11-21T10:00:00Z"
    })
}

fn sample_attribute_def_json() -> serde_json::Value {
    json!({
        "id": "018f0000-0000-7000-8000-000000000080",
        "tenant_id": "018f0000-0000-7000-8000-000000000001",
        "category_id": "018f0000-0000-7000-8000-000000000070",
        "key": "potencia_rms",
        "label": "Potência RMS",
        "attribute_type": "number",
        "required": false,
        "unit": "W",
        "options": null,
        "display_order": 1,
        "created_at": "2026-11-21T10:00:00Z",
        "updated_at": "2026-11-21T10:00:00Z"
    })
}

fn sample_model_json() -> serde_json::Value {
    json!({
        "id": "018f0000-0000-7000-8000-000000000090",
        "tenant_id": "018f0000-0000-7000-8000-000000000001",
        "brand_id": "018f0000-0000-7000-8000-000000000060",
        "category_id": "018f0000-0000-7000-8000-000000000070",
        "name": "CDJ-3000",
        "model_code": "CDJ3000",
        "control_mode": "serialized",
        "active": true,
        "barcode_ean_gtin": null,
        "default_daily_price_cents": 25000,
        "weight_kg": 5.5,
        "power_consumption_w": 40,
        "setup_minutes": 10,
        "strike_minutes": 10,
        "default_notes": null,
        "search_text": "pioneer dj cdj-3000 cdj3000",
        "created_at": "2026-11-21T10:00:00Z",
        "updated_at": "2026-11-21T10:00:00Z"
    })
}

fn sample_media_view_json() -> serde_json::Value {
    json!({
        "id": "018f0000-0000-7000-8000-000000000100",
        "tenant_id": "018f0000-0000-7000-8000-000000000001",
        "model_id": "018f0000-0000-7000-8000-000000000090",
        "asset_id": "018f0000-0000-7000-8000-000000000101",
        "kind": "image",
        "blob_id": "018f0000-0000-7000-8000-000000000102",
        "blob_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "mime_type": "image/webp",
        "thumb_blob_id": null,
        "thumb_sha256": null,
        "thumb_mime_type": null,
        "title": "Foto Frontal",
        "doc_type": null,
        "origin": "local_upload",
        "credit_note": null,
        "language": null,
        "version_note": null,
        "original_filename": "cdj3000_front.webp",
        "width": 1920,
        "height": 1080,
        "byte_size": 154200,
        "is_primary": true,
        "sort_order": 1,
        "created_at": "2026-11-21T10:00:00Z",
        "updated_at": "2026-11-21T10:00:00Z"
    })
}

pub fn get_fixtures_dir() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("Raiz do projeto deve existir")
        .join("src")
        .join("lib")
        .join("data")
        .join("fixtures")
        .join("contracts")
}

pub fn sync_or_verify_contract_fixtures(update: bool) -> Result<(), String> {
    let fixtures_dir = get_fixtures_dir();
    std::fs::create_dir_all(&fixtures_dir)
        .map_err(|e| format!("Falha ao criar diretório de fixtures: {e}"))?;

    let registry_commands: HashSet<String> = COMMAND_REGISTRY
        .iter()
        .map(|c| c.name.to_string())
        .collect();

    for cmd in COMMAND_REGISTRY {
        let fixture = build_fixture_for_command(cmd.name).ok_or_else(|| {
            format!(
                "Comando '{}' registrado em COMMAND_REGISTRY não possui definição de fixture de contrato em contracts.rs!",
                cmd.name
            )
        })?;

        let file_path = fixtures_dir.join(format!("{}.json", cmd.name));
        let expected_json = serde_json::to_string_pretty(&fixture)
            .map_err(|e| format!("Falha ao serializar fixture para {}: {e}", cmd.name))?;

        if update {
            std::fs::write(&file_path, &expected_json)
                .map_err(|e| format!("Falha ao gravar fixture {}: {e}", file_path.display()))?;
        } else {
            if !file_path.exists() {
                return Err(format!(
                    "Fixture de contrato ausente para o comando '{}' ({})! Execute com EVENTRENT_UPDATE_CONTRACTS=1 para gerar.",
                    cmd.name,
                    file_path.display()
                ));
            }

            let disk_content = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Falha ao ler fixture {}: {e}", file_path.display()))?;

            let disk_fixture: ContractFixture = serde_json::from_str(&disk_content)
                .map_err(|e| format!("Fixture inválida em {}: {e}", file_path.display()))?;

            if disk_fixture != fixture {
                return Err(format!(
                    "Fixture de contrato desatualizada para o comando '{}'! O contrato no código diverge do arquivo {}. Execute com EVENTRENT_UPDATE_CONTRACTS=1 para atualizar.",
                    cmd.name,
                    file_path.display()
                ));
            }
        }
    }

    // 2.3: Garante que não existem fixtures órfãs (arquivos que não correspondem a comandos em COMMAND_REGISTRY)
    if let Ok(entries) = std::fs::read_dir(&fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !registry_commands.contains(stem) {
                        return Err(format!(
                            "Fixture órfã detectada em {}: o comando '{}' não está presente em COMMAND_REGISTRY!",
                            path.display(),
                            stem
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_commands_have_up_to_date_contract_fixtures() {
        let update = std::env::var("EVENTRENT_UPDATE_CONTRACTS").unwrap_or_default() == "1";
        sync_or_verify_contract_fixtures(update)
            .expect("Verificação de fixtures de contrato falhou");
    }
}
