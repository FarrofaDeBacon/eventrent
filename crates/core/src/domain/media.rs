use serde::{Deserialize, Serialize};

use crate::domain::validation::is_dangerous_or_invisible_control_char;
use crate::error::{CoreError, Result};

pub const MAX_DISPLAY_IMAGE_DIMENSION: u32 = 2048;
pub const MAX_DISPLAY_IMAGE_BYTES: usize = 600 * 1024;
pub const MAX_THUMB_IMAGE_DIMENSION: u32 = 512;
pub const MAX_THUMB_IMAGE_BYTES: usize = 60 * 1024;
pub const MAX_PDF_DOCUMENT_BYTES: usize = 25 * 1024 * 1024;
pub const MAX_PHOTOS_PER_MODEL: usize = 12;
pub const MAX_DOCUMENTS_PER_MODEL: usize = 20;
pub const MAX_MEDIA_TITLE_CHARS: usize = 150;
pub const MAX_MEDIA_NOTE_CHARS: usize = 300;
pub const MAX_MEDIA_FILENAME_CHARS: usize = 150;
pub const MAX_MEDIA_LANGUAGE_CHARS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaMimeType {
    #[serde(rename = "image/webp")]
    ImageWebp,
    #[serde(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/png")]
    ImagePng,
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
}

impl MediaMimeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ImageWebp => "image/webp",
            Self::ImageJpeg => "image/jpeg",
            Self::ImagePng => "image/png",
            Self::ApplicationPdf => "application/pdf",
        }
    }

    pub fn parse_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "image/webp" => Ok(Self::ImageWebp),
            "image/jpeg" => Ok(Self::ImageJpeg),
            "image/png" => Ok(Self::ImagePng),
            "application/pdf" => Ok(Self::ApplicationPdf),
            other => Err(CoreError::Validation(format!(
                "Tipo MIME não suportado: '{other}'. Permitidos: image/webp, image/jpeg, image/png, application/pdf."
            ))),
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::ImageWebp | Self::ImageJpeg | Self::ImagePng)
    }

    pub fn is_pdf(&self) -> bool {
        matches!(self, Self::ApplicationPdf)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Document,
}

impl MediaKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Document => "document",
        }
    }

    pub fn parse_str(value: &str) -> Result<Self> {
        match value.trim() {
            "image" => Ok(Self::Image),
            "document" => Ok(Self::Document),
            other => Err(CoreError::Validation(format!(
                "Tipo de mídia inválido: '{other}'. Use 'image' ou 'document'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaDocType {
    Manual,
    Datasheet,
    Certificate,
    Other,
}

impl MediaDocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Datasheet => "datasheet",
            Self::Certificate => "certificate",
            Self::Other => "other",
        }
    }

    pub fn parse_str(value: &str) -> Result<Self> {
        match value.trim() {
            "manual" => Ok(Self::Manual),
            "datasheet" => Ok(Self::Datasheet),
            "certificate" => Ok(Self::Certificate),
            "other" => Ok(Self::Other),
            other => Err(CoreError::Validation(format!(
                "Tipo de documento inválido: '{other}'. Use 'manual', 'datasheet', 'certificate' ou 'other'."
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaOrigin {
    Manufacturer,
    Own,
    Website,
    Other,
}

impl MediaOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manufacturer => "manufacturer",
            Self::Own => "own",
            Self::Website => "website",
            Self::Other => "other",
        }
    }

    pub fn parse_str(value: &str) -> Result<Self> {
        match value.trim() {
            "manufacturer" => Ok(Self::Manufacturer),
            "own" => Ok(Self::Own),
            "website" => Ok(Self::Website),
            "other" => Ok(Self::Other),
            other => Err(CoreError::Validation(format!(
                "Origem de mídia inválida: '{other}'. Use 'manufacturer', 'own', 'website' ou 'other'."
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaBlob {
    pub id: String,
    pub tenant_id: String,
    pub sha256: String,
    pub mime_type: MediaMimeType,
    pub byte_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub storage_key: String,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: String,
    pub tenant_id: String,
    pub kind: MediaKind,
    pub blob_id: String,
    pub thumb_blob_id: Option<String>,
    pub title: String,
    pub doc_type: Option<MediaDocType>,
    pub origin: MediaOrigin,
    pub credit_note: Option<String>,
    pub language: Option<String>,
    pub version_note: Option<String>,
    pub original_filename: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub byte_size: i64,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentModelMedia {
    pub id: String,
    pub tenant_id: String,
    pub model_id: String,
    pub asset_id: String,
    pub is_primary: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentModelMediaView {
    pub id: String,
    pub tenant_id: String,
    pub model_id: String,
    pub asset_id: String,
    pub kind: MediaKind,
    pub blob_id: String,
    pub blob_sha256: String,
    pub mime_type: MediaMimeType,
    pub thumb_blob_id: Option<String>,
    pub thumb_sha256: Option<String>,
    pub thumb_mime_type: Option<MediaMimeType>,
    pub title: String,
    pub doc_type: Option<MediaDocType>,
    pub origin: MediaOrigin,
    pub credit_note: Option<String>,
    pub language: Option<String>,
    pub version_note: Option<String>,
    pub original_filename: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub byte_size: i64,
    pub is_primary: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedImagePayload {
    pub mime_type: MediaMimeType,
    pub sha256: String,
    pub byte_size: i64,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPdfPayload {
    pub mime_type: MediaMimeType,
    pub sha256: String,
    pub byte_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachModelPhotoInput {
    pub model_id: String,
    pub title: String,
    pub origin: MediaOrigin,
    pub credit_note: Option<String>,
    pub original_filename: String,
    pub display_mime_type: String,
    pub display_bytes: Vec<u8>,
    pub thumb_mime_type: String,
    pub thumb_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachModelDocumentInput {
    pub model_id: String,
    pub title: String,
    pub doc_type: MediaDocType,
    pub origin: MediaOrigin,
    pub credit_note: Option<String>,
    pub language: Option<String>,
    pub version_note: Option<String>,
    pub original_filename: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateModelMediaInput {
    pub media_link_id: String,
    pub title: String,
    pub doc_type: Option<MediaDocType>,
    pub origin: MediaOrigin,
    pub credit_note: Option<String>,
    pub language: Option<String>,
    pub version_note: Option<String>,
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderModelMediaInput {
    pub model_id: String,
    pub kind: MediaKind,
    pub ordered_link_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaGarbageCollectionResult {
    pub deleted_assets: usize,
    pub deleted_blobs: usize,
    pub deleted_files: usize,
}

use sha2::{Digest, Sha256};

/// Calcula o hash SHA-256 (FIPS 180-4) utilizando a crate consolidada `sha2` (ADR-0015).
/// Retorna 64 caracteres hexadecimais minúsculos sem cópia excessiva.
pub fn compute_sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in result {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Calcula o hash SHA-256 de forma assíncrona, offloadando para `tokio::task::spawn_blocking`
/// para arquivos médios e grandes (evita bloquear o loop do Tokio).
/// B2: Propaga erros do JoinHandle (`Result<String, CoreError>`) em vez de retornar string vazia em caso de falha.
pub async fn compute_sha256_hex_async(data: Vec<u8>) -> Result<String> {
    if data.len() < 32 * 1024 {
        Ok(compute_sha256_hex(&data))
    } else {
        tokio::task::spawn_blocking(move || compute_sha256_hex(&data))
            .await
            .map_err(|e| {
                CoreError::Internal(format!(
                    "Falha na execução de tarefa de hash assíncrono: {e}"
                ))
            })
    }
}

pub fn validate_sha256_hex(sha256: &str) -> Result<String> {
    let trimmed = sha256.trim().to_ascii_lowercase();
    if trimmed.len() != 64
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(CoreError::Validation(
            "Hash SHA-256 inválido: deve conter exatamente 64 caracteres hexadecimais.".to_string(),
        ));
    }
    Ok(trimmed)
}

pub fn validate_tenant_storage_segment(tenant_id: &str) -> Result<String> {
    let trimmed = tenant_id.trim();
    if trimmed.is_empty()
        || trimmed.len() > 64
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(CoreError::Validation(
            "Identificador de tenant inválido para armazenamento de mídia.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

pub fn build_media_storage_key(tenant_id: &str, sha256: &str) -> Result<String> {
    let clean_tenant = validate_tenant_storage_segment(tenant_id)?;
    let clean_sha = validate_sha256_hex(sha256)?;
    let prefix = &clean_sha[..2];
    Ok(format!("{clean_tenant}/{prefix}/{clean_sha}"))
}

/// Detecta o tipo MIME real a partir dos magic bytes do cabeçalho.
pub fn detect_magic_mime(bytes: &[u8]) -> Option<MediaMimeType> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(MediaMimeType::ImageWebp);
    }
    if bytes.len() >= 3 && bytes[0..3] == [0xFF, 0xD8, 0xFF] {
        return Some(MediaMimeType::ImageJpeg);
    }
    if bytes.len() >= 8 && bytes[0..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some(MediaMimeType::ImagePng);
    }
    if bytes.len() >= 5 && &bytes[0..5] == b"%PDF-" {
        return Some(MediaMimeType::ApplicationPdf);
    }
    None
}

/// Rejeita imagens que contenham metadados EXIF, XMP, IPTC ou comentários (R6):
/// - WebP: chunk EXIF, chunk "XMP " e flags EXIF (0x08) ou XMP (0x04) do VP8X.
/// - JPEG: APP1 (qualquer Exif/XMP), APP13 (IPTC/Photoshop) e COM (comentário).
/// - PNG: chunks eXIf, tEXt, iTXt e zTXt.
///
/// Também valida a integridade estrutural básica dos chunks/segmentos contra arquivos truncados.
pub fn reject_image_metadata(mime: MediaMimeType, bytes: &[u8]) -> Result<()> {
    match mime {
        MediaMimeType::ImageWebp => inspect_webp_no_metadata(bytes),
        MediaMimeType::ImageJpeg => inspect_jpeg_no_metadata(bytes),
        MediaMimeType::ImagePng => inspect_png_no_metadata(bytes),
        MediaMimeType::ApplicationPdf => Ok(()),
    }
}

pub fn reject_image_exif(mime: MediaMimeType, bytes: &[u8]) -> Result<()> {
    reject_image_metadata(mime, bytes)
}

fn inspect_webp_no_metadata(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 20 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err(CoreError::Validation(
            "Arquivo WebP truncado ou malformado.".to_string(),
        ));
    }

    let riff_declared_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if riff_declared_size + 8 > bytes.len() {
        return Err(CoreError::Validation(
            "Arquivo WebP truncado: tamanho RIFF menor que o declarado no cabeçalho.".to_string(),
        ));
    }

    let limit = riff_declared_size + 8;
    let mut pos = 12;
    while pos < limit {
        if pos + 8 > limit {
            return Err(CoreError::Validation(
                "Arquivo WebP truncado ao ler cabeçalho de chunk.".to_string(),
            ));
        }
        let fourcc = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        let data_start = pos + 8;
        let data_end = data_start
            .checked_add(chunk_size)
            .ok_or_else(|| CoreError::Validation("Chunk WebP com tamanho inválido.".to_string()))?;
        if data_end > limit {
            return Err(CoreError::Validation(
                "Arquivo WebP truncado dentro de um chunk.".to_string(),
            ));
        }

        if fourcc == b"EXIF" {
            return Err(CoreError::Validation(
                "Imagem WebP contém metadados EXIF (chunk EXIF), que são proibidos. Remova o EXIF antes de enviar.".to_string(),
            ));
        }

        if fourcc == b"XMP " {
            return Err(CoreError::Validation(
                "Imagem WebP contém metadados XMP (chunk XMP ), que são proibidos. Remova os metadados antes de enviar.".to_string(),
            ));
        }

        if fourcc == b"VP8X" {
            if chunk_size < 10 {
                return Err(CoreError::Validation(
                    "Chunk VP8X do WebP truncado.".to_string(),
                ));
            }
            let flags = bytes[data_start];
            // Bit 3 (0x08) = flag EXIF
            if (flags & 0x08) != 0 {
                return Err(CoreError::Validation(
                    "Imagem WebP indica presença de metadados EXIF no cabeçalho VP8X.".to_string(),
                ));
            }
            // Bit 2 (0x04) = flag XMP
            if (flags & 0x04) != 0 {
                return Err(CoreError::Validation(
                    "Imagem WebP indica presença de metadados XMP no cabeçalho VP8X.".to_string(),
                ));
            }
        }

        let padded_size = chunk_size + (chunk_size & 1);
        pos = data_start + padded_size;
    }

    Ok(())
}

fn inspect_jpeg_no_metadata(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 4 || bytes[0..2] != [0xFF, 0xD8] {
        return Err(CoreError::Validation(
            "Arquivo JPEG truncado ou sem marcador SOI.".to_string(),
        ));
    }

    let mut pos = 2;
    let mut saw_frame_or_scan = false;

    while pos < bytes.len() {
        if bytes[pos] != 0xFF {
            return Err(CoreError::Validation(
                "Arquivo JPEG malformado: marcador de segmento inválido.".to_string(),
            ));
        }

        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            return Err(CoreError::Validation(
                "Arquivo JPEG truncado após prefixo de marcador.".to_string(),
            ));
        }

        let marker = bytes[pos];
        pos += 1;

        if marker == 0xD9 {
            // EOI
            return Ok(());
        }
        if marker == 0xDA {
            // SOS (início dos dados comprimidos da imagem)
            saw_frame_or_scan = true;
            break;
        }
        if (0xD0..=0xD8).contains(&marker) || marker == 0x01 {
            continue;
        }

        if pos + 2 > bytes.len() {
            return Err(CoreError::Validation(
                "Arquivo JPEG truncado ao ler tamanho do segmento.".to_string(),
            ));
        }
        let seg_len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if seg_len < 2 || pos + seg_len > bytes.len() {
            return Err(CoreError::Validation(
                "Arquivo JPEG truncado: segmento excede o tamanho do arquivo.".to_string(),
            ));
        }

        // Rejeita qualquer segmento APP1 (0xE1: EXIF, XMP, etc.)
        if marker == 0xE1 {
            return Err(CoreError::Validation(
                "Imagem JPEG contém metadados em segmento APP1 (EXIF/XMP), que são proibidos. Remova os metadados antes de enviar.".to_string(),
            ));
        }

        // Rejeita segmento APP13 (0xED: Photoshop / IPTC)
        if marker == 0xED {
            return Err(CoreError::Validation(
                "Imagem JPEG contém metadados em segmento APP13 (IPTC/Photoshop), que são proibidos. Remova os metadados antes de enviar.".to_string(),
            ));
        }

        // Rejeita segmento COM (0xFE: Comentário)
        if marker == 0xFE {
            return Err(CoreError::Validation(
                "Imagem JPEG contém segmento de comentário (COM), que é proibido. Remova os comentários antes de enviar.".to_string(),
            ));
        }

        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            saw_frame_or_scan = true;
        }

        pos += seg_len;
    }

    if !saw_frame_or_scan {
        return Err(CoreError::Validation(
            "Arquivo JPEG truncado: nenhum quadro de imagem encontrado.".to_string(),
        ));
    }

    Ok(())
}

fn inspect_png_no_metadata(bytes: &[u8]) -> Result<()> {
    const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 33 || bytes[0..8] != PNG_SIG {
        return Err(CoreError::Validation(
            "Arquivo PNG truncado ou assinatura inválida.".to_string(),
        ));
    }

    let mut pos = 8;
    let mut saw_iend = false;

    while pos < bytes.len() {
        if pos + 12 > bytes.len() {
            return Err(CoreError::Validation(
                "Arquivo PNG truncado ao ler cabeçalho de chunk.".to_string(),
            ));
        }
        let chunk_len =
            u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                as usize;
        let chunk_type = &bytes[pos + 4..pos + 8];
        let next_pos = pos
            .checked_add(12)
            .and_then(|p| p.checked_add(chunk_len))
            .ok_or_else(|| CoreError::Validation("Chunk PNG com tamanho inválido.".to_string()))?;

        if next_pos > bytes.len() {
            return Err(CoreError::Validation(
                "Arquivo PNG truncado dentro de um chunk.".to_string(),
            ));
        }

        if chunk_type == b"eXIf" {
            return Err(CoreError::Validation(
                "Imagem PNG contém metadados EXIF (chunk eXIf), que são proibidos. Remova o EXIF antes de enviar.".to_string(),
            ));
        }

        if chunk_type == b"tEXt" {
            return Err(CoreError::Validation(
                "Imagem PNG contém metadados textuais (chunk tEXt), que são proibidos. Remova os metadados antes de enviar.".to_string(),
            ));
        }

        if chunk_type == b"iTXt" {
            return Err(CoreError::Validation(
                "Imagem PNG contém metadados textuais internacionais (chunk iTXt), que são proibidos. Remova os metadados antes de enviar.".to_string(),
            ));
        }

        if chunk_type == b"zTXt" {
            return Err(CoreError::Validation(
                "Imagem PNG contém metadados textuais comprimidos (chunk zTXt), que são proibidos. Remova os metadados antes de enviar.".to_string(),
            ));
        }

        if chunk_type == b"IEND" {
            saw_iend = true;
            break;
        }

        pos = next_pos;
    }

    if !saw_iend {
        return Err(CoreError::Validation(
            "Arquivo PNG truncado: chunk IEND ausente.".to_string(),
        ));
    }

    Ok(())
}

fn validate_image_common(
    role_label: &str,
    declared_mime: &str,
    bytes: &[u8],
    max_bytes: usize,
    max_dimension: u32,
) -> Result<ValidatedImagePayload> {
    if bytes.is_empty() {
        return Err(CoreError::Validation(format!(
            "O arquivo de {role_label} está vazio."
        )));
    }
    if bytes.len() > max_bytes {
        return Err(CoreError::Validation(format!(
            "O arquivo de {role_label} excede o tamanho máximo permitido de {} KB (recebido: {} bytes).",
            max_bytes / 1024,
            bytes.len()
        )));
    }

    let parsed_mime = MediaMimeType::parse_str(declared_mime)?;
    if !parsed_mime.is_image() {
        return Err(CoreError::Validation(format!(
            "Tipo MIME inválido para {role_label}: '{}'. Esperado image/webp, image/jpeg ou image/png.",
            parsed_mime.as_str()
        )));
    }

    let magic_mime = detect_magic_mime(bytes).ok_or_else(|| {
        CoreError::Validation(format!(
            "Cabeçalho binário (magic bytes) não reconhecido ou truncado para {role_label}."
        ))
    })?;

    if magic_mime != parsed_mime {
        return Err(CoreError::Validation(format!(
            "O conteúdo binário ({}) não corresponde ao tipo MIME declarado ({}) em {role_label}.",
            magic_mime.as_str(),
            parsed_mime.as_str()
        )));
    }

    reject_image_exif(parsed_mime, bytes)?;

    let size = imagesize::blob_size(bytes).map_err(|_| {
        CoreError::Validation(format!(
            "Não foi possível ler as dimensões do cabeçalho da imagem ({role_label}); arquivo truncado ou corrompido."
        ))
    })?;

    let width = u32::try_from(size.width).unwrap_or(0);
    let height = u32::try_from(size.height).unwrap_or(0);
    if width == 0 || height == 0 {
        return Err(CoreError::Validation(format!(
            "Dimensões inválidas ({width}x{height}) para {role_label}."
        )));
    }

    let max_side = width.max(height);
    if max_side > max_dimension {
        return Err(CoreError::Validation(format!(
            "A imagem de {role_label} ({width}x{height} px) excede a dimensão máxima permitida de {max_dimension} px no maior lado."
        )));
    }

    Ok(ValidatedImagePayload {
        mime_type: parsed_mime,
        sha256: compute_sha256_hex(bytes),
        byte_size: bytes.len() as i64,
        width: width as i32,
        height: height as i32,
    })
}

/// Valida imagem de exibição (WebP/JPEG/PNG, sem EXIF, até 2048 px no maior lado e até 600 KB).
pub fn validate_display_image(declared_mime: &str, bytes: &[u8]) -> Result<ValidatedImagePayload> {
    validate_image_common(
        "imagem de exibição",
        declared_mime,
        bytes,
        MAX_DISPLAY_IMAGE_BYTES,
        MAX_DISPLAY_IMAGE_DIMENSION,
    )
}

/// Valida miniatura de imagem (WebP/JPEG/PNG, sem EXIF, até 512 px no maior lado e até 60 KB).
pub fn validate_thumbnail_image(
    declared_mime: &str,
    bytes: &[u8],
) -> Result<ValidatedImagePayload> {
    validate_image_common(
        "miniatura",
        declared_mime,
        bytes,
        MAX_THUMB_IMAGE_BYTES,
        MAX_THUMB_IMAGE_DIMENSION,
    )
}

/// Valida documento PDF (magic "%PDF-", coerente com application/pdf, até 25 MB).
/// B2: Calcula o hash SHA-256 de forma assíncrona via `compute_sha256_hex_async` em tarefa `spawn_blocking`.
pub async fn validate_pdf_document(
    declared_mime: &str,
    bytes: &[u8],
) -> Result<ValidatedPdfPayload> {
    if bytes.is_empty() {
        return Err(CoreError::Validation(
            "O arquivo PDF está vazio.".to_string(),
        ));
    }
    if bytes.len() > MAX_PDF_DOCUMENT_BYTES {
        return Err(CoreError::Validation(format!(
            "O documento PDF excede o limite máximo de 25 MB (recebido: {} bytes).",
            bytes.len()
        )));
    }

    let parsed_mime = MediaMimeType::parse_str(declared_mime)?;
    if !parsed_mime.is_pdf() {
        return Err(CoreError::Validation(format!(
            "Tipo MIME inválido para documento: '{}'. Apenas 'application/pdf' é aceito.",
            parsed_mime.as_str()
        )));
    }

    let magic_mime = detect_magic_mime(bytes).ok_or_else(|| {
        CoreError::Validation(
            "Cabeçalho binário inválido: o arquivo não inicia com a assinatura '%PDF-'."
                .to_string(),
        )
    })?;

    if magic_mime != MediaMimeType::ApplicationPdf {
        return Err(CoreError::Validation(format!(
            "O conteúdo do arquivo ({}) não corresponde ao MIME 'application/pdf'.",
            magic_mime.as_str()
        )));
    }

    if bytes.len() < 8 {
        return Err(CoreError::Validation(
            "Arquivo PDF truncado ou incompleto.".to_string(),
        ));
    }

    let sha256 = compute_sha256_hex_async(bytes.to_vec()).await?;

    Ok(ValidatedPdfPayload {
        mime_type: MediaMimeType::ApplicationPdf,
        sha256,
        byte_size: bytes.len() as i64,
    })
}

/// Valida limite de fotos (12) ou documentos (20) por modelo de equipamento.
pub fn validate_model_media_quota(kind: MediaKind, current_count: usize) -> Result<()> {
    match kind {
        MediaKind::Image => {
            if current_count >= MAX_PHOTOS_PER_MODEL {
                return Err(CoreError::Validation(format!(
                    "Limite máximo de {MAX_PHOTOS_PER_MODEL} fotos por modelo de equipamento atingido."
                )));
            }
        }
        MediaKind::Document => {
            if current_count >= MAX_DOCUMENTS_PER_MODEL {
                return Err(CoreError::Validation(format!(
                    "Limite máximo de {MAX_DOCUMENTS_PER_MODEL} documentos por modelo de equipamento atingido."
                )));
            }
        }
    }
    Ok(())
}

/// Nomes reservados de dispositivos no Windows que nunca podem ser usados como nomes de arquivos ou stems.
/// B4: Inclui também variantes com dígitos sobrescritos (COM¹..³, LPT¹..³) tratadas como reservadas no Windows 11.
pub const WINDOWS_RESERVED_NAMES: [&str; 28] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "COM¹", "COM²", "COM³", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
    "LPT9", "LPT¹", "LPT²", "LPT³",
];

/// Valida o título da mídia (1 a 150 caracteres, sem caracteres de controle, invisíveis ou bidi).
pub fn validate_media_title(title: &str) -> Result<String> {
    let trimmed = title.trim();
    let count = trimmed.chars().count();
    if count == 0 || count > MAX_MEDIA_TITLE_CHARS {
        return Err(CoreError::Validation(format!(
            "O título da mídia deve ter entre 1 e {MAX_MEDIA_TITLE_CHARS} caracteres."
        )));
    }
    if trimmed.chars().any(is_dangerous_or_invisible_control_char) {
        return Err(CoreError::Validation(
            "O título da mídia não pode conter caracteres de controle, invisíveis ou de direção de texto.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Valida `credit_note` ou `version_note` (opcional, até 300 caracteres, sem caracteres de controle, invisíveis ou bidi).
pub fn validate_optional_media_note(
    field_label: &str,
    note: Option<&str>,
) -> Result<Option<String>> {
    let Some(raw) = note else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_MEDIA_NOTE_CHARS {
        return Err(CoreError::Validation(format!(
            "O campo '{field_label}' deve ter no máximo {MAX_MEDIA_NOTE_CHARS} caracteres."
        )));
    }
    if trimmed.chars().any(is_dangerous_or_invisible_control_char) {
        return Err(CoreError::Validation(format!(
            "O campo '{field_label}' não pode conter caracteres de controle, invisíveis ou de direção de texto."
        )));
    }
    Ok(Some(trimmed.to_string()))
}

/// Valida idioma opcional do documento (ex.: pt-BR, en, es; até 20 caracteres, sem controle, invisíveis ou bidi).
pub fn validate_optional_media_language(language: Option<&str>) -> Result<Option<String>> {
    let Some(raw) = language else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_MEDIA_LANGUAGE_CHARS
        || trimmed.chars().any(is_dangerous_or_invisible_control_char)
    {
        return Err(CoreError::Validation(format!(
            "O idioma do documento deve ter no máximo {MAX_MEDIA_LANGUAGE_CHARS} caracteres e nenhum caractere de controle, invisível ou bidi."
        )));
    }
    Ok(Some(trimmed.to_string()))
}

/// Saneia e valida o nome original do arquivo (sem caminho, sem caracteres de controle/invisíveis/bidi, até 150 caracteres).
pub fn sanitize_media_filename(raw: &str) -> Result<String> {
    if raw.chars().any(is_dangerous_or_invisible_control_char) {
        return Err(CoreError::Validation(
            "O nome do arquivo não pode conter caracteres de controle, invisíveis ou de direção de texto.".to_string(),
        ));
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(
            "O nome do arquivo não pode ser vazio.".to_string(),
        ));
    }

    if trimmed.contains("..") {
        return Err(CoreError::Validation(
            "O nome do arquivo contém sequência de caminho inválida ('..').".to_string(),
        ));
    }

    // Remove qualquer prefixo de diretório (Windows ou POSIX) para garantir apenas o nome base.
    let base = trimmed
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('.');

    if base.is_empty() {
        return Err(CoreError::Validation(
            "Nome de arquivo inválido após remoção de caminho.".to_string(),
        ));
    }

    let sanitized: String = base
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            other => other,
        })
        .collect();

    let char_count = sanitized.chars().count();
    if char_count == 0 || char_count > MAX_MEDIA_FILENAME_CHARS {
        return Err(CoreError::Validation(format!(
            "O nome do arquivo deve ter entre 1 e {MAX_MEDIA_FILENAME_CHARS} caracteres."
        )));
    }

    Ok(sanitized)
}

/// Gera um stem de arquivo seguro a partir do título do documento (R1.1).
///
/// Regras aplicadas:
/// 1. Rejeita imediatamente caracteres de controle, invisíveis ou de direção bidi (R2).
/// 2. Filtra aceitando apenas letras, números, espaço, hífen '-', sublinhado '_' e ponto '.'.
/// 3. Trunca em no máximo 60 caracteres.
/// 4. Remove pontos e espaços finais (evita restrições e bugs do sistema de arquivos do Windows).
/// 5. Se o resultado for vazio ou coincidir com nomes reservados do Windows (CON, PRN, AUX, NUL, COM1..9, LPT1..9),
///    retorna o padrão "documento".
pub fn sanitize_safe_document_stem(raw_title: &str) -> Result<String> {
    if raw_title
        .chars()
        .any(is_dangerous_or_invisible_control_char)
    {
        return Err(CoreError::Validation(
            "O título do documento contém caracteres de controle, invisíveis ou de direção de texto inválidos.".to_string(),
        ));
    }

    let trimmed = raw_title.trim();
    // Extrai o nome base se houver prefixo de caminho (Windows ou POSIX)
    let base = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed).trim();

    // Se o título fornecido já contiver extensão .pdf (case insensitive), remove para evitar duplicação
    let without_pdf = if base.to_ascii_lowercase().ends_with(".pdf") {
        &base[..base.len() - 4]
    } else {
        base
    };

    // B4: Verifica se o nome base já corresponde a um nome reservado do Windows (incluindo COM¹..³ e LPT¹..³)
    let raw_upper = without_pdf.trim().to_uppercase();
    let raw_base = raw_upper.split('.').next().unwrap_or(&raw_upper);
    if WINDOWS_RESERVED_NAMES.contains(&raw_upper.as_str())
        || WINDOWS_RESERVED_NAMES.contains(&raw_base)
    {
        return Ok("documento".to_string());
    }

    // Substitui caracteres especiais de arquivo por '_' antes de filtrar
    let pre_sanitized: String = without_pdf
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            other => other,
        })
        .collect();

    // B4: Restringe o stem a letras Unicode (is_alphabetic) e dígitos estritamente ASCII (is_ascii_digit),
    // excluindo dígitos não ASCII (como sobrescritos ¹, ², ³ ou numerais de outros scripts).
    let filtered: String = pre_sanitized
        .chars()
        .filter(|&c| c.is_alphabetic() || c.is_ascii_digit() || matches!(c, ' ' | '-' | '_' | '.'))
        .take(60)
        .collect();

    let stem = filtered.trim_end_matches(['.', ' ']).trim();

    if stem.is_empty() {
        return Ok("documento".to_string());
    }

    let upper = stem.to_uppercase();
    let base_name = upper.split('.').next().unwrap_or(&upper);
    if WINDOWS_RESERVED_NAMES.contains(&upper.as_str())
        || WINDOWS_RESERVED_NAMES.contains(&base_name)
    {
        return Ok("documento".to_string());
    }

    Ok(stem.to_string())
}

/// Gera um nome de arquivo de documento estritamente seguro com extensão fixa ".pdf" (R1.1 e R1.2).
pub fn safe_document_filename(raw_title: &str) -> Result<String> {
    let stem = sanitize_safe_document_stem(raw_title)?;
    Ok(format!("{stem}.pdf"))
}

/// Normaliza o `original_filename` persistido no banco ao anexar mídia (R1.3).
///
/// Garante que:
/// - O nome de arquivo é saneado e livre de caracteres de controle/invisíveis/bidi.
/// - Documentos terminam estritamente em `.pdf`.
/// - Imagens terminam na extensão canônica exata de seu MIME (`.webp`, `.jpg` ou `.png`).
/// - Extensões perigosas enviadas pelo usuário (ex: `.bat`, `.cmd`, `.exe`) são completamente descartadas.
/// - Nomes reservados do Windows (ex: `CON`, `NUL`) são substituídos por "documento" ou "imagem".
pub fn normalize_media_filename(raw_filename: &str, mime: MediaMimeType) -> Result<String> {
    let sanitized = sanitize_media_filename(raw_filename)?;

    let ext = match mime {
        MediaMimeType::ImageWebp => "webp",
        MediaMimeType::ImageJpeg => "jpg",
        MediaMimeType::ImagePng => "png",
        MediaMimeType::ApplicationPdf => "pdf",
    };

    // Extrai o stem removendo a última extensão, se houver
    let stem = match sanitized.rfind('.') {
        Some(idx) if idx > 0 => &sanitized[..idx],
        Some(0) => "", // Caso inicie com ponto, ex.: ".pdf"
        _ => {
            // Se não contém ponto, mas coincide com extensão comum isolada (ex: arquivo enviado como "pdf" ou ".pdf" que teve o ponto removido no sanitize)
            if sanitized.eq_ignore_ascii_case("pdf")
                || sanitized.eq_ignore_ascii_case("webp")
                || sanitized.eq_ignore_ascii_case("jpg")
                || sanitized.eq_ignore_ascii_case("jpeg")
                || sanitized.eq_ignore_ascii_case("png")
            {
                ""
            } else {
                &sanitized
            }
        }
    };

    let trimmed_stem = stem.trim_end_matches(['.', ' ']).trim();
    let fallback = match mime {
        MediaMimeType::ApplicationPdf => "documento",
        _ => "imagem",
    };

    let base_stem = if trimmed_stem.is_empty() {
        fallback
    } else {
        let upper = trimmed_stem.to_uppercase();
        let base_name = upper.split('.').next().unwrap_or(&upper);
        if WINDOWS_RESERVED_NAMES.contains(&upper.as_str())
            || WINDOWS_RESERVED_NAMES.contains(&base_name)
        {
            fallback
        } else {
            trimmed_stem
        }
    };

    Ok(format!("{base_stem}.{ext}"))
}
