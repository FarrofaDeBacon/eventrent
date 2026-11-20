use std::path::Path;

use eventrent_core::{
    safe_document_filename, AttachModelDocumentInput, AttachModelPhotoInput,
    EquipmentModelMediaView, MediaDocType, MediaGarbageCollectionResult, MediaOrigin,
    ReorderModelMediaInput, UpdateModelMediaInput, MAX_DISPLAY_IMAGE_BYTES, MAX_PDF_DOCUMENT_BYTES,
    MAX_THUMB_IMAGE_BYTES,
};
use serde::Deserialize;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

use crate::guards::require_session;
use crate::AppState;

const MAX_FRAME_HEADER_BYTES: usize = 16 * 1024;
const MAX_PHOTO_IPC_FRAME_BYTES: usize =
    MAX_DISPLAY_IMAGE_BYTES + MAX_THUMB_IMAGE_BYTES + MAX_FRAME_HEADER_BYTES + 4;
const MAX_DOCUMENT_IPC_FRAME_BYTES: usize = MAX_PDF_DOCUMENT_BYTES + MAX_FRAME_HEADER_BYTES + 4;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhotoFrameHeader {
    model_id: String,
    title: String,
    origin: MediaOrigin,
    credit_note: Option<String>,
    original_filename: String,
    display_mime_type: String,
    display_byte_len: usize,
    thumb_mime_type: String,
    thumb_byte_len: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentFrameHeader {
    model_id: String,
    title: String,
    doc_type: MediaDocType,
    origin: MediaOrigin,
    credit_note: Option<String>,
    language: Option<String>,
    version_note: Option<String>,
    original_filename: String,
    mime_type: String,
    byte_len: usize,
}

pub fn decode_photo_ipc_frame(raw: &[u8]) -> Result<AttachModelPhotoInput, String> {
    if raw.len() < 4 {
        return Err("Payload binário de foto inválido ou truncado.".to_string());
    }
    if raw.len() > MAX_PHOTO_IPC_FRAME_BYTES {
        return Err(format!(
            "Payload binário de foto excede o limite máximo permitido no canal IPC ({} KB).",
            MAX_PHOTO_IPC_FRAME_BYTES / 1024
        ));
    }

    let header_len = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
    if header_len == 0 || header_len > MAX_FRAME_HEADER_BYTES || 4 + header_len > raw.len() {
        return Err("Cabeçalho de metadados da foto inválido ou excede 16 KB.".to_string());
    }

    let header: PhotoFrameHeader = serde_json::from_slice(&raw[4..4 + header_len])
        .map_err(|e| format!("Falha ao decodificar metadados da foto: {e}"))?;

    if header.display_byte_len == 0 || header.display_byte_len > MAX_DISPLAY_IMAGE_BYTES {
        return Err(format!(
            "A imagem de exibição excede o limite de {} KB antes do processamento.",
            MAX_DISPLAY_IMAGE_BYTES / 1024
        ));
    }
    if header.thumb_byte_len == 0 || header.thumb_byte_len > MAX_THUMB_IMAGE_BYTES {
        return Err(format!(
            "A miniatura excede o limite de {} KB antes do processamento.",
            MAX_THUMB_IMAGE_BYTES / 1024
        ));
    }

    let expected_total = 4usize
        .checked_add(header_len)
        .and_then(|v| v.checked_add(header.display_byte_len))
        .and_then(|v| v.checked_add(header.thumb_byte_len))
        .ok_or_else(|| "Tamanho total de frame de foto inválido.".to_string())?;

    if raw.len() != expected_total {
        return Err(format!(
            "Tamanho do frame binário de foto ({}) diverge do esperado ({}).",
            raw.len(),
            expected_total
        ));
    }

    let display_start = 4 + header_len;
    let display_end = display_start + header.display_byte_len;
    let thumb_end = display_end + header.thumb_byte_len;

    Ok(AttachModelPhotoInput {
        model_id: header.model_id,
        title: header.title,
        origin: header.origin,
        credit_note: header.credit_note,
        original_filename: header.original_filename,
        display_mime_type: header.display_mime_type,
        display_bytes: raw[display_start..display_end].to_vec(),
        thumb_mime_type: header.thumb_mime_type,
        thumb_bytes: raw[display_end..thumb_end].to_vec(),
    })
}

pub fn decode_document_ipc_frame(raw: &[u8]) -> Result<AttachModelDocumentInput, String> {
    if raw.len() < 4 {
        return Err("Payload binário de documento inválido ou truncado.".to_string());
    }
    if raw.len() > MAX_DOCUMENT_IPC_FRAME_BYTES {
        return Err(format!(
            "Payload binário de documento excede o limite máximo de {} MB antes do processamento.",
            MAX_PDF_DOCUMENT_BYTES / (1024 * 1024)
        ));
    }

    let header_len = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
    if header_len == 0 || header_len > MAX_FRAME_HEADER_BYTES || 4 + header_len > raw.len() {
        return Err("Cabeçalho de metadados do documento inválido ou excede 16 KB.".to_string());
    }

    let header: DocumentFrameHeader = serde_json::from_slice(&raw[4..4 + header_len])
        .map_err(|e| format!("Falha ao decodificar metadados do documento: {e}"))?;

    if header.byte_len == 0 || header.byte_len > MAX_PDF_DOCUMENT_BYTES {
        return Err(format!(
            "O documento PDF excede o limite de {} MB antes do processamento.",
            MAX_PDF_DOCUMENT_BYTES / (1024 * 1024)
        ));
    }

    let expected_total = 4usize
        .checked_add(header_len)
        .and_then(|v| v.checked_add(header.byte_len))
        .ok_or_else(|| "Tamanho total de frame de documento inválido.".to_string())?;

    if raw.len() != expected_total {
        return Err(format!(
            "Tamanho do frame binário de documento ({}) diverge do esperado ({}).",
            raw.len(),
            expected_total
        ));
    }

    let pdf_start = 4 + header_len;
    let pdf_end = pdf_start + header.byte_len;

    Ok(AttachModelDocumentInput {
        model_id: header.model_id,
        title: header.title,
        doc_type: header.doc_type,
        origin: header.origin,
        credit_note: header.credit_note,
        language: header.language,
        version_note: header.version_note,
        original_filename: header.original_filename,
        mime_type: header.mime_type,
        bytes: raw[pdf_start..pdf_end].to_vec(),
    })
}

#[tauri::command]
pub async fn list_model_media(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<Vec<EquipmentModelMediaView>, String> {
    let session = require_session(&state)?;
    state
        .services
        .list_model_media(&session, &model_id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn get_media_blob_bytes(
    state: State<'_, AppState>,
    blob_id: String,
) -> Result<Response, String> {
    let session = require_session(&state)?;
    let (_blob, bytes) = state
        .services
        .get_media_blob_bytes(&session, &blob_id)
        .await
        .map_err(crate::map_core_err)?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub async fn attach_model_photo(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<EquipmentModelMediaView, String> {
    let session = require_session(&state)?;
    let raw_bytes = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        InvokeBody::Json(_) => {
            return Err(
                "Transferência de foto exige canal binário raw (Uint8Array/ArrayBuffer) do Tauri v2."
                    .to_string(),
            );
        }
    };

    let input = decode_photo_ipc_frame(raw_bytes)?;
    state
        .services
        .attach_model_photo(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn attach_model_document(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<EquipmentModelMediaView, String> {
    let session = require_session(&state)?;
    let raw_bytes = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        InvokeBody::Json(_) => {
            return Err(
                "Transferência de documento exige canal binário raw (Uint8Array/ArrayBuffer) do Tauri v2."
                    .to_string(),
            );
        }
    };

    let input = decode_document_ipc_frame(raw_bytes)?;
    state
        .services
        .attach_model_document(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn update_model_media_metadata(
    state: State<'_, AppState>,
    input: UpdateModelMediaInput,
) -> Result<EquipmentModelMediaView, String> {
    let session = require_session(&state)?;
    state
        .services
        .update_model_media_metadata(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn reorder_model_media(
    state: State<'_, AppState>,
    input: ReorderModelMediaInput,
) -> Result<Vec<EquipmentModelMediaView>, String> {
    let session = require_session(&state)?;
    state
        .services
        .reorder_model_media(&session, &input)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn set_primary_model_photo(
    state: State<'_, AppState>,
    media_link_id: String,
) -> Result<EquipmentModelMediaView, String> {
    let session = require_session(&state)?;
    state
        .services
        .set_primary_model_photo(&session, &media_link_id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn remove_model_media(
    state: State<'_, AppState>,
    media_link_id: String,
) -> Result<(), String> {
    let session = require_session(&state)?;
    state
        .services
        .remove_model_media(&session, &media_link_id)
        .await
        .map_err(crate::map_core_err)
}

#[tauri::command]
pub async fn collect_unreferenced_media(
    state: State<'_, AppState>,
) -> Result<MediaGarbageCollectionResult, String> {
    let session = require_session(&state)?;
    state
        .services
        .collect_unreferenced_media(&session)
        .await
        .map_err(crate::map_core_err)
}

/// Grava uma cópia temporária com o nome saneado e seguro (<stem seguro>.pdf)
/// na pasta temporária do app e retorna o caminho completo do arquivo temporário gerado.
/// A extensão do arquivo é SEMPRE ".pdf", nunca vindo do usuário (R1.1).
pub fn write_sanitized_temp_document(
    temp_media_root: &Path,
    title_or_name: &str,
    bytes: &[u8],
) -> Result<std::path::PathBuf, String> {
    let filename = safe_document_filename(title_or_name).map_err(crate::map_core_err)?;
    let slot_dir = temp_media_root.join(Uuid::now_v7().to_string());
    std::fs::create_dir_all(&slot_dir).map_err(|e| {
        format!(
            "Falha ao criar pasta temporária de visualização ({}): {e}",
            slot_dir.display()
        )
    })?;

    let target_path = slot_dir.join(&filename);
    std::fs::write(&target_path, bytes).map_err(|e| {
        format!(
            "Falha ao gravar cópia temporária do documento ({}): {e}",
            target_path.display()
        )
    })?;

    // Garante que o caminho temporário gerado termina estritamente em .pdf e está dentro da pasta temporária do app
    if !target_path
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
    {
        return Err(
            "Violação de segurança: arquivo temporário não possui extensão .pdf.".to_string(),
        );
    }

    let canonical_root = temp_media_root
        .canonicalize()
        .map_err(|e| format!("Falha ao resolver caminho canônico raiz: {e}"))?;
    let canonical_target = target_path
        .canonicalize()
        .map_err(|e| format!("Falha ao resolver caminho canônico do arquivo temporário: {e}"))?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err(
            "Violação de segurança: caminho temporário fora da pasta de mídia temporária do app."
                .to_string(),
        );
    }

    Ok(target_path)
}

#[tauri::command]
pub async fn open_model_document_external(
    app: AppHandle,
    state: State<'_, AppState>,
    media_link_id: String,
) -> Result<String, String> {
    let session = require_session(&state)?;
    let (view, bytes) = state
        .services
        .get_model_document_for_external(&session, &media_link_id)
        .await
        .map_err(crate::map_core_err)?;

    // Usa o título saneado como stem seguro com extensão estritamente .pdf
    let temp_path = write_sanitized_temp_document(&state.temp_media_dir, &view.title, &bytes)?;
    let path_str = temp_path.to_string_lossy().to_string();

    app.opener()
        .open_path(path_str.clone(), None::<&str>)
        .map_err(|e| format!("Falha ao abrir documento no leitor do sistema: {e}"))?;

    Ok(path_str)
}

#[tauri::command]
pub async fn save_model_document_as(
    app: AppHandle,
    state: State<'_, AppState>,
    media_link_id: String,
) -> Result<Option<String>, String> {
    let session = require_session(&state)?;
    let (view, bytes) = state
        .services
        .get_model_document_for_external(&session, &media_link_id)
        .await
        .map_err(crate::map_core_err)?;

    // Nome sugerido sempre no padrão <stem seguro>.pdf mantendo filtro PDF (R1.2)
    let suggested_name = safe_document_filename(&view.title).map_err(crate::map_core_err)?;
    let app_clone = app.clone();

    let picked = tokio::task::spawn_blocking(move || {
        app_clone
            .dialog()
            .file()
            .set_file_name(&suggested_name)
            .add_filter("Documento PDF", &["pdf"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("Falha ao aguardar diálogo de salvamento: {e}"))?;

    let Some(file_path) = picked else {
        return Ok(None);
    };

    let mut dest_path = file_path
        .into_path()
        .map_err(|e| format!("Caminho de destino inválido: {e}"))?;

    // Garante que o arquivo salvo tenha extensão estritamente .pdf, nunca vindo do usuário
    if !dest_path
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
    {
        dest_path.set_extension("pdf");
    }

    std::fs::write(&dest_path, &bytes).map_err(|e| {
        format!(
            "Falha ao salvar documento em '{}': {e}",
            dest_path.display()
        )
    })?;

    Ok(Some(dest_path.to_string_lossy().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_sanitized_temp_document_r1_security_cases() {
        let temp_dir =
            std::env::temp_dir().join(format!("eventrent_test_media_{}", Uuid::now_v7()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let dummy_pdf = b"%PDF-1.7\ntest\n%%EOF";
        let cases = [
            "evil.bat",
            "x.cmd",
            "x.ps1",
            "x.js",
            "x.vbs",
            "x.hta",
            "x.url",
            "x.lnk",
            "x.scr",
            "x.exe",
            "a.pdf.bat",
            "manual.pdf ",
            ".pdf",
            "CON",
            "con.pdf",
            "nul.pdf",
            "",
        ];

        let canonical_root = temp_dir.canonicalize().unwrap();

        for case in cases {
            let path = write_sanitized_temp_document(&temp_dir, case, dummy_pdf).unwrap();
            let path_str = path.to_string_lossy();
            assert!(
                path_str.ends_with(".pdf"),
                "Caminho temporário para {case:?} deve terminar com .pdf: {path_str}"
            );
            assert!(
                !path_str.ends_with(".bat")
                    && !path_str.ends_with(".cmd")
                    && !path_str.ends_with(".ps1")
                    && !path_str.ends_with(".exe")
                    && !path_str.ends_with(".url")
                    && !path_str.ends_with(".lnk"),
                "Extensão perigosa mantida para {case:?}: {path_str}"
            );
            let canonical_target = path.canonicalize().unwrap();
            assert!(
                canonical_target.starts_with(&canonical_root),
                "Caminho {canonical_target:?} deve estar dentro da pasta raiz {canonical_root:?}"
            );
        }

        // Teste de 300 caracteres
        let long_300 = "a".repeat(300);
        let path = write_sanitized_temp_document(&temp_dir, &long_300, dummy_pdf).unwrap();
        assert!(path.to_string_lossy().ends_with(".pdf"));
        let canonical_target = path.canonicalize().unwrap();
        assert!(canonical_target.starts_with(&canonical_root));

        // Limpeza do diretório de teste
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
