use serde::{Deserialize, Serialize};

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

/// Implementação pura de SHA-256 (FIPS 180-4) sem dependências externas.
/// Retorna 64 caracteres hexadecimais minúsculos.
#[allow(clippy::needless_range_loop)]
pub fn compute_sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    let (chunks, _) = padded.as_chunks::<64>();
    for chunk in chunks {
        for i in 0..16 {
            let offset = i * 4;
            w[i] = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    format!(
        "{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
        h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]
    )
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

/// Rejeita imagens que contenham metadados EXIF (WebP chunk EXIF / flag VP8X, JPEG APP1 Exif, PNG eXIf).
/// Também valida a integridade estrutural básica dos chunks/segmentos contra arquivos truncados.
pub fn reject_image_exif(mime: MediaMimeType, bytes: &[u8]) -> Result<()> {
    match mime {
        MediaMimeType::ImageWebp => inspect_webp_no_exif(bytes),
        MediaMimeType::ImageJpeg => inspect_jpeg_no_exif(bytes),
        MediaMimeType::ImagePng => inspect_png_no_exif(bytes),
        MediaMimeType::ApplicationPdf => Ok(()),
    }
}

fn inspect_webp_no_exif(bytes: &[u8]) -> Result<()> {
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

        if fourcc == b"VP8X" {
            if chunk_size < 10 {
                return Err(CoreError::Validation(
                    "Chunk VP8X do WebP truncado.".to_string(),
                ));
            }
            // Bit 3 (0x08) do primeiro byte de flags do VP8X indica presença de EXIF
            if (bytes[data_start] & 0x08) != 0 {
                return Err(CoreError::Validation(
                    "Imagem WebP indica presença de metadados EXIF no cabeçalho VP8X.".to_string(),
                ));
            }
        }

        let padded_size = chunk_size + (chunk_size & 1);
        pos = data_start + padded_size;
    }

    Ok(())
}

fn inspect_jpeg_no_exif(bytes: &[u8]) -> Result<()> {
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

        let seg_data = &bytes[pos + 2..pos + seg_len];
        if marker == 0xE1 && (seg_data.starts_with(b"Exif\0\0") || seg_data.starts_with(b"Exif\0"))
        {
            return Err(CoreError::Validation(
                "Imagem JPEG contém metadados EXIF (segmento APP1 Exif), que são proibidos. Remova o EXIF antes de enviar.".to_string(),
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

fn inspect_png_no_exif(bytes: &[u8]) -> Result<()> {
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
pub fn validate_pdf_document(declared_mime: &str, bytes: &[u8]) -> Result<ValidatedPdfPayload> {
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

    Ok(ValidatedPdfPayload {
        mime_type: MediaMimeType::ApplicationPdf,
        sha256: compute_sha256_hex(bytes),
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

/// Valida o título da mídia (1 a 150 caracteres, sem caracteres de controle).
pub fn validate_media_title(title: &str) -> Result<String> {
    let trimmed = title.trim();
    let count = trimmed.chars().count();
    if count == 0 || count > MAX_MEDIA_TITLE_CHARS {
        return Err(CoreError::Validation(format!(
            "O título da mídia deve ter entre 1 e {MAX_MEDIA_TITLE_CHARS} caracteres."
        )));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(
            "O título da mídia não pode conter caracteres de controle.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Valida `credit_note` ou `version_note` (opcional, até 300 caracteres, sem caracteres de controle).
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
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(format!(
            "O campo '{field_label}' não pode conter caracteres de controle."
        )));
    }
    Ok(Some(trimmed.to_string()))
}

/// Valida idioma opcional do documento (ex.: pt-BR, en, es; até 20 caracteres).
pub fn validate_optional_media_language(language: Option<&str>) -> Result<Option<String>> {
    let Some(raw) = language else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_MEDIA_LANGUAGE_CHARS || trimmed.chars().any(|c| c.is_control())
    {
        return Err(CoreError::Validation(format!(
            "O idioma do documento deve ter no máximo {MAX_MEDIA_LANGUAGE_CHARS} caracteres e nenhum caractere de controle."
        )));
    }
    Ok(Some(trimmed.to_string()))
}

/// Saneia e valida o nome original do arquivo (sem caminho, sem caracteres de controle, até 150 caracteres).
pub fn sanitize_media_filename(raw: &str) -> Result<String> {
    if raw.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(
            "O nome do arquivo não pode conter caracteres de controle.".to_string(),
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
