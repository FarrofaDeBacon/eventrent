import type {
  AttachModelDocumentInput,
  AttachModelPhotoInput,
} from "$lib/data/catalog-repository";

/**
 * Empacota metadados e bytes da foto (exibição + miniatura) em um único frame binário
 * `Uint8Array` para envio direto pelo canal raw `Request` do Tauri v2 (sem Base64).
 */
export function encodePhotoIpcFrame(input: AttachModelPhotoInput): Uint8Array {
  const headerObj = {
    modelId: input.model_id,
    title: input.title,
    origin: input.origin,
    creditNote: input.credit_note,
    originalFilename: input.original_filename,
    displayMimeType: input.display_mime_type,
    displayByteLen: input.display_bytes.byteLength,
    thumbMimeType: input.thumb_mime_type,
    thumbByteLen: input.thumb_bytes.byteLength,
  };
  const headerBytes = new TextEncoder().encode(JSON.stringify(headerObj));
  const totalLen =
    4 +
    headerBytes.byteLength +
    input.display_bytes.byteLength +
    input.thumb_bytes.byteLength;

  const frame = new Uint8Array(totalLen);
  const dv = new DataView(frame.buffer, frame.byteOffset, frame.byteLength);
  dv.setUint32(0, headerBytes.byteLength, true);

  let offset = 4;
  frame.set(headerBytes, offset);
  offset += headerBytes.byteLength;

  frame.set(input.display_bytes, offset);
  offset += input.display_bytes.byteLength;

  frame.set(input.thumb_bytes, offset);
  return frame;
}

/**
 * Empacota metadados e bytes de um documento PDF em um único frame binário
 * `Uint8Array` para envio direto pelo canal raw `Request` do Tauri v2 (sem Base64).
 */
export function encodeDocumentIpcFrame(
  input: AttachModelDocumentInput,
): Uint8Array {
  const headerObj = {
    modelId: input.model_id,
    title: input.title,
    docType: input.doc_type,
    origin: input.origin,
    creditNote: input.credit_note,
    language: input.language,
    versionNote: input.version_note,
    originalFilename: input.original_filename,
    mimeType: input.mime_type,
    byteLen: input.bytes.byteLength,
  };
  const headerBytes = new TextEncoder().encode(JSON.stringify(headerObj));
  const totalLen = 4 + headerBytes.byteLength + input.bytes.byteLength;

  const frame = new Uint8Array(totalLen);
  const dv = new DataView(frame.buffer, frame.byteOffset, frame.byteLength);
  dv.setUint32(0, headerBytes.byteLength, true);

  let offset = 4;
  frame.set(headerBytes, offset);
  offset += headerBytes.byteLength;

  frame.set(input.bytes, offset);
  return frame;
}
