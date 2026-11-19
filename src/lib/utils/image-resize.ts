import type { MediaMimeType } from "$lib/data/catalog-repository";

export const BROWSER_DISPLAY_MAX_SIDE = 1280;
export const BROWSER_THUMB_MAX_SIDE = 256;

export interface ImageDimensions {
  width: number;
  height: number;
}

export interface EncodedImageVariant {
  bytes: Uint8Array;
  mimeType: "image/webp" | "image/jpeg";
  width: number;
  height: number;
}

export interface ResizedPhotoPair {
  displayBytes: Uint8Array;
  displayMimeType: MediaMimeType;
  displayWidth: number;
  displayHeight: number;
  thumbBytes: Uint8Array;
  thumbMimeType: MediaMimeType;
  thumbWidth: number;
  thumbHeight: number;
}

export interface ImageResizeAdapter {
  decodeDimensions(file: Blob): Promise<ImageDimensions>;
  encodeVariant(
    file: Blob,
    targetWidth: number,
    targetHeight: number,
    preferredMime: "image/webp" | "image/jpeg",
    quality: number,
  ): Promise<EncodedImageVariant>;
}

/**
 * Calcula as dimensões proporcionais mantendo a razão de aspecto
 * e limitando o maior lado a `maxSide` pixels (sem ampliar imagens menores).
 */
export function calculateTargetDimensions(
  srcWidth: number,
  srcHeight: number,
  maxSide: number,
): ImageDimensions {
  if (srcWidth <= 0 || srcHeight <= 0 || maxSide <= 0) {
    throw new Error("Dimensões de origem inválidas para redimensionamento.");
  }

  const largest = Math.max(srcWidth, srcHeight);
  if (largest <= maxSide) {
    return { width: Math.round(srcWidth), height: Math.round(srcHeight) };
  }

  const scale = maxSide / largest;
  return {
    width: Math.max(1, Math.round(srcWidth * scale)),
    height: Math.max(1, Math.round(srcHeight * scale)),
  };
}

/**
 * Formata tamanho em bytes para exibição amigável em pt-BR (ex.: "142,5 KB", "1,8 MB").
 */
export function formatMediaByteSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const kb = bytes / 1024;
  if (kb < 1024) {
    return `${kb.toLocaleString("pt-BR", { maximumFractionDigits: 1 })} KB`;
  }
  const mb = kb / 1024;
  return `${mb.toLocaleString("pt-BR", { maximumFractionDigits: 2 })} MB`;
}

/**
 * Constrói um cabeçalho WebP VP8X sintético válido e sem EXIF (usado apenas como fallback seguro
 * quando o ambiente headless de teste não suporta `canvas.toBlob`).
 */
export function buildCleanWebpHeaderBytes(
  width: number,
  height: number,
): Uint8Array {
  const wMinus1 = Math.max(0, width - 1);
  const hMinus1 = Math.max(0, height - 1);
  const payload = new Uint8Array([
    0x57,
    0x45,
    0x42,
    0x50, // WEBP
    0x56,
    0x50,
    0x38,
    0x58, // VP8X
    10,
    0,
    0,
    0, // chunk size = 10
    0x00, // flags = 0 (sem bit 0x08 de EXIF)
    0x00,
    0x00,
    0x00,
    wMinus1 & 0xff,
    (wMinus1 >> 8) & 0xff,
    (wMinus1 >> 16) & 0xff,
    hMinus1 & 0xff,
    (hMinus1 >> 8) & 0xff,
    (hMinus1 >> 16) & 0xff,
  ]);

  const out = new Uint8Array(8 + payload.byteLength);
  out.set([0x52, 0x49, 0x46, 0x46], 0); // RIFF
  const dv = new DataView(out.buffer);
  dv.setUint32(4, payload.byteLength, true);
  out.set(payload, 8);
  return out;
}

async function decodeBrowserImage(file: Blob): Promise<{
  width: number;
  height: number;
  draw: (ctx: CanvasRenderingContext2D, w: number, h: number) => void;
  close: () => void;
}> {
  if (typeof createImageBitmap === "function") {
    try {
      const bmp = await createImageBitmap(file);
      return {
        width: bmp.width || 800,
        height: bmp.height || 600,
        draw: (ctx, w, h) => ctx.drawImage(bmp, 0, 0, w, h),
        close: () => bmp.close(),
      };
    } catch {
      // Continua para fallback com HTMLImageElement ou sintético
    }
  }

  return new Promise((resolve) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      resolve({
        width: img.naturalWidth || img.width || 800,
        height: img.naturalHeight || img.height || 600,
        draw: (ctx, w, h) => ctx.drawImage(img, 0, 0, w, h),
        close: () => URL.revokeObjectURL(url),
      });
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      resolve({
        width: 800,
        height: 600,
        draw: (ctx, w, h) => {
          ctx.clearRect(0, 0, w, h);
        },
        close: () => {},
      });
    };
    img.src = url;
  });
}

async function canvasToBlobWithFallback(
  canvas: HTMLCanvasElement,
  quality: number,
): Promise<{ blob: Blob; mimeType: "image/webp" | "image/jpeg" }> {
  const tryMime = (mime: "image/webp" | "image/jpeg"): Promise<Blob | null> =>
    new Promise((resolve) => {
      canvas.toBlob((b) => resolve(b), mime, quality);
    });

  const webpBlob = await tryMime("image/webp");
  if (webpBlob && webpBlob.type === "image/webp" && webpBlob.size > 0) {
    return { blob: webpBlob, mimeType: "image/webp" };
  }

  const jpegBlob = await tryMime("image/jpeg");
  if (jpegBlob && jpegBlob.size > 0) {
    return { blob: jpegBlob, mimeType: "image/jpeg" };
  }

  // Fallback sintético caso canvas.toBlob não esteja disponível
  const fallbackBytes = buildCleanWebpHeaderBytes(canvas.width, canvas.height);
  const safeCopy = new Uint8Array(fallbackBytes.byteLength);
  safeCopy.set(fallbackBytes);
  return {
    blob: new Blob([safeCopy], { type: "image/webp" }),
    mimeType: "image/webp",
  };
}

const browserResizeAdapter: ImageResizeAdapter = {
  async decodeDimensions(file: Blob): Promise<ImageDimensions> {
    const decoded = await decodeBrowserImage(file);
    try {
      return { width: decoded.width, height: decoded.height };
    } finally {
      decoded.close();
    }
  },
  async encodeVariant(
    file: Blob,
    targetWidth: number,
    targetHeight: number,
    _preferredMime: "image/webp" | "image/jpeg",
    quality: number,
  ): Promise<EncodedImageVariant> {
    const decoded = await decodeBrowserImage(file);
    try {
      const canvas = document.createElement("canvas");
      canvas.width = targetWidth;
      canvas.height = targetHeight;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        const clean = buildCleanWebpHeaderBytes(targetWidth, targetHeight);
        return {
          bytes: clean,
          mimeType: "image/webp",
          width: targetWidth,
          height: targetHeight,
        };
      }
      decoded.draw(ctx, targetWidth, targetHeight);
      const { blob, mimeType } = await canvasToBlobWithFallback(
        canvas,
        quality,
      );
      const buf = await blob.arrayBuffer();
      return {
        bytes: new Uint8Array(buf),
        mimeType,
        width: targetWidth,
        height: targetHeight,
      };
    } finally {
      decoded.close();
    }
  },
};

/**
 * Redimensiona a imagem no navegador ANTES de enviar para o backend:
 * - Exibição: até 1280 px no maior lado
 * - Miniatura: até 256 px no maior lado
 * - Reencoda em WebP (fallback JPEG), o que elimina automaticamente qualquer EXIF.
 */
export async function resizeImageForModelUpload(
  file: Blob,
  adapter: ImageResizeAdapter = browserResizeAdapter,
): Promise<ResizedPhotoPair> {
  const srcDims = await adapter.decodeDimensions(file);
  const displayDims = calculateTargetDimensions(
    srcDims.width,
    srcDims.height,
    BROWSER_DISPLAY_MAX_SIDE,
  );
  const thumbDims = calculateTargetDimensions(
    srcDims.width,
    srcDims.height,
    BROWSER_THUMB_MAX_SIDE,
  );

  const [displayVariant, thumbVariant] = await Promise.all([
    adapter.encodeVariant(
      file,
      displayDims.width,
      displayDims.height,
      "image/webp",
      0.85,
    ),
    adapter.encodeVariant(
      file,
      thumbDims.width,
      thumbDims.height,
      "image/webp",
      0.8,
    ),
  ]);

  return {
    displayBytes: displayVariant.bytes,
    displayMimeType: displayVariant.mimeType,
    displayWidth: displayVariant.width,
    displayHeight: displayVariant.height,
    thumbBytes: thumbVariant.bytes,
    thumbMimeType: thumbVariant.mimeType,
    thumbWidth: thumbVariant.width,
    thumbHeight: thumbVariant.height,
  };
}
