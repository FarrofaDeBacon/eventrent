import { describe, expect, it, vi } from "vitest";
import {
  extractFilenameStem,
  matchFilenameToCatalogModels,
  type BatchMatchableModel,
} from "./batch-photo-matcher";
import {
  calculateTargetDimensions,
  resizeImageForModelUpload,
  type ImageResizeAdapter,
} from "./image-resize";
import { MediaBlobUrlLruCache } from "./media-lru-cache";

describe("Casamento de nomes de arquivo na importação em lote (SKU -> código -> EAN)", () => {
  const models: BatchMatchableModel[] = [
    {
      id: "m-cdj",
      name: "Pioneer CDJ-3000",
      sku: "DJ-CDJ-3000",
      model_code: "CDJ-3000",
      ean: "4573201242006",
    },
    {
      id: "m-overlap",
      name: "Modelo com Código Igual ao SKU de Outro",
      sku: "OUTRO-SKU-99",
      model_code: "DJ-CDJ-3000", // igual ao SKU do m-cdj para provar precedência SKU > model_code
      ean: "7891234567895",
    },
    {
      id: "m-amb-1",
      name: "Par LED 18x18W Marca A",
      sku: "LUZ-PAR-A",
      model_code: "PAR-18X18",
      ean: null,
    },
    {
      id: "m-amb-2",
      name: "Par LED 18x18W Marca B",
      sku: "LUZ-PAR-B",
      model_code: "par-18x18",
      ean: null,
    },
  ];

  it("extrai o nome base sem extensão corretamente", () => {
    expect(extractFilenameStem("C:\\fotos\\DJ-CDJ-3000.webp")).toBe(
      "DJ-CDJ-3000",
    );
    expect(extractFilenameStem("/tmp/7891234567895.foto.final.jpg")).toBe(
      "7891234567895.foto.final",
    );
  });

  it("prioriza casamento por SKU antes de código do modelo e EAN", () => {
    const res = matchFilenameToCatalogModels("dj-cdj-3000.webp", models);
    expect(res.status).toBe("matched");
    expect(res.matchedBy).toBe("sku");
    expect(res.matchedModel?.id).toBe("m-cdj");
  });

  it("casa por código do modelo quando não há SKU correspondente", () => {
    const res = matchFilenameToCatalogModels("cdj-3000.png", models);
    expect(res.status).toBe("matched");
    expect(res.matchedBy).toBe("model_code");
    expect(res.matchedModel?.id).toBe("m-cdj");
  });

  it("casa por EAN quando não há SKU nem código do modelo correspondente", () => {
    const res = matchFilenameToCatalogModels("4573201242006.jpg", models);
    expect(res.status).toBe("matched");
    expect(res.matchedBy).toBe("ean");
    expect(res.matchedModel?.id).toBe("m-cdj");
  });

  it("marca como ambíguo quando múltiplos modelos empatam no mesmo campo", () => {
    const res = matchFilenameToCatalogModels("PAR-18X18.webp", models);
    expect(res.status).toBe("ambiguous");
    expect(res.matchedBy).toBe("model_code");
    expect(res.matchedModel).toBeNull();
    expect(res.candidates.map((c) => c.id)).toEqual(["m-amb-1", "m-amb-2"]);
  });

  it("marca como sem correspondência quando não casa com SKU, código nem EAN", () => {
    const res = matchFilenameToCatalogModels(
      "arquivo-desconhecido.webp",
      models,
    );
    expect(res.status).toBe("unmatched");
    expect(res.matchedBy).toBeNull();
    expect(res.matchedModel).toBeNull();
  });
});

describe("Helper de redimensionamento de imagem no navegador (com mock)", () => {
  it("calcula dimensões proporcionais respeitando o limite de 1280 px e 256 px", () => {
    expect(calculateTargetDimensions(4000, 3000, 1280)).toEqual({
      width: 1280,
      height: 960,
    });
    expect(calculateTargetDimensions(4000, 3000, 256)).toEqual({
      width: 256,
      height: 192,
    });
    // Imagens menores não são ampliadas
    expect(calculateTargetDimensions(600, 400, 1280)).toEqual({
      width: 600,
      height: 400,
    });
  });

  it("gera versão de exibição (1280 px) e miniatura (256 px) usando o adapter mockado", async () => {
    const encodeSpy = vi.fn(
      async (
        _file: Blob,
        targetWidth: number,
        targetHeight: number,
        preferredMime: "image/webp" | "image/jpeg",
      ) => ({
        bytes: new Uint8Array([targetWidth & 0xff, targetHeight & 0xff]),
        mimeType: preferredMime,
        width: targetWidth,
        height: targetHeight,
      }),
    );

    const mockAdapter: ImageResizeAdapter = {
      decodeDimensions: vi.fn(async () => ({ width: 2560, height: 1280 })),
      encodeVariant: encodeSpy,
    };

    const dummyBlob = new Blob([new Uint8Array([1, 2, 3])], {
      type: "image/png",
    });
    const result = await resizeImageForModelUpload(dummyBlob, mockAdapter);

    expect(result.displayWidth).toBe(1280);
    expect(result.displayHeight).toBe(640);
    expect(result.displayMimeType).toBe("image/webp");
    expect(result.thumbWidth).toBe(256);
    expect(result.thumbHeight).toBe(128);
    expect(result.thumbMimeType).toBe("image/webp");
    expect(encodeSpy).toHaveBeenCalledTimes(2);
  });
});

describe("Cache LRU de Blob URLs em memória (evicção e revokeObjectURL)", () => {
  it("evicta o item menos recentemente usado por contagem máxima e revoga a Blob URL", () => {
    let counter = 0;
    const revoked: string[] = [];
    const cache = new MediaBlobUrlLruCache({
      maxItems: 2,
      maxBytes: 1024 * 1024,
      createObjectUrl: () => `blob:test-url-${++counter}`,
      revokeObjectUrl: (url) => revoked.push(url),
    });

    const u1 = cache.put("b1", new Uint8Array(100), "image/webp");
    const u2 = cache.put("b2", new Uint8Array(100), "image/webp");
    expect(u1).toBe("blob:test-url-1");
    expect(u2).toBe("blob:test-url-2");

    // Acessa "b1" para torná-lo o mais recentemente usado (b2 passa a ser o mais antigo)
    expect(cache.get("b1")).toBe("blob:test-url-1");

    // Inserir "b3" deve evictar "b2" e chamar revokeObjectURL("blob:test-url-2")
    const u3 = cache.put("b3", new Uint8Array(100), "image/webp");
    expect(u3).toBe("blob:test-url-3");
    expect(cache.get("b2")).toBeUndefined();
    expect(cache.get("b1")).toBe("blob:test-url-1");
    expect(revoked).toEqual(["blob:test-url-2"]);
  });

  it("evicta itens antigos quando o limite total de bytes é ultrapassado e libera tudo no clear()", () => {
    let counter = 0;
    const revoked: string[] = [];
    const cache = new MediaBlobUrlLruCache({
      maxItems: 10,
      maxBytes: 250,
      createObjectUrl: () => `blob:bytes-url-${++counter}`,
      revokeObjectUrl: (url) => revoked.push(url),
    });

    cache.put("b1", new Uint8Array(120), "image/webp");
    cache.put("b2", new Uint8Array(120), "image/webp");
    expect(cache.currentBytes).toBe(240);

    // Inserir "b3" com 120 bytes excede 250 -> evicta "b1"
    cache.put("b3", new Uint8Array(120), "image/webp");
    expect(cache.get("b1")).toBeUndefined();
    expect(cache.currentBytes).toBe(240);
    expect(revoked).toEqual(["blob:bytes-url-1"]);

    cache.clear();
    expect(cache.size).toBe(0);
    expect(cache.currentBytes).toBe(0);
    expect(revoked).toEqual([
      "blob:bytes-url-1",
      "blob:bytes-url-2",
      "blob:bytes-url-3",
    ]);
  });
});
