export interface MediaLruCacheOptions {
  maxItems?: number;
  maxBytes?: number;
  createObjectUrl?: (blob: Blob) => string;
  revokeObjectUrl?: (url: string) => void;
}

interface CacheEntry {
  url: string;
  byteSize: number;
  mimeType: string;
}

const DEFAULT_MAX_ITEMS = 150;
const DEFAULT_MAX_BYTES = 32 * 1024 * 1024; // 32 MB

/**
 * Cache LRU em memória para Blob URLs de mídia (ADR-0013).
 * Controla limite de itens e limite total de bytes, revogando automaticamente
 * `URL.revokeObjectURL` na evicção ou limpeza.
 */
export class MediaBlobUrlLruCache {
  private readonly maxItems: number;
  private readonly maxBytes: number;
  private readonly createUrl: (blob: Blob) => string;
  private readonly revokeUrl: (url: string) => void;
  private readonly entries = new Map<string, CacheEntry>();
  private readonly inFlight = new Map<string, Promise<string>>();
  private totalBytes = 0;

  constructor(options: MediaLruCacheOptions = {}) {
    this.maxItems = Math.max(1, options.maxItems ?? DEFAULT_MAX_ITEMS);
    this.maxBytes = Math.max(1, options.maxBytes ?? DEFAULT_MAX_BYTES);
    this.createUrl =
      options.createObjectUrl ?? ((blob: Blob) => URL.createObjectURL(blob));
    this.revokeUrl =
      options.revokeObjectUrl ?? ((url: string) => URL.revokeObjectURL(url));
  }

  get size(): number {
    return this.entries.size;
  }

  get currentBytes(): number {
    return this.totalBytes;
  }

  get(key: string): string | undefined {
    const existing = this.entries.get(key);
    if (!existing) {
      return undefined;
    }
    // Move para o final do Map (mais recentemente usado)
    this.entries.delete(key);
    this.entries.set(key, existing);
    return existing.url;
  }

  put(
    key: string,
    bytes: Uint8Array | ArrayBuffer,
    mimeType = "image/webp",
  ): string {
    const prev = this.entries.get(key);
    if (prev) {
      this.entries.delete(key);
      this.totalBytes = Math.max(0, this.totalBytes - prev.byteSize);
      this.revokeUrl(prev.url);
    }

    const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    const byteSize = view.byteLength;

    while (
      this.entries.size > 0 &&
      (this.entries.size >= this.maxItems ||
        this.totalBytes + byteSize > this.maxBytes)
    ) {
      const oldestKey = this.entries.keys().next().value;
      if (oldestKey === undefined) break;
      this.evictKey(oldestKey);
    }

    const safeCopy = new Uint8Array(view.byteLength);
    safeCopy.set(view);
    const blob = new Blob([safeCopy], { type: mimeType });
    const url = this.createUrl(blob);

    this.entries.set(key, { url, byteSize, mimeType });
    this.totalBytes += byteSize;
    return url;
  }

  async getOrLoad(
    key: string,
    mimeType: string,
    loader: () => Promise<Uint8Array | ArrayBuffer>,
  ): Promise<string> {
    const cached = this.get(key);
    if (cached !== undefined) {
      return cached;
    }

    const pending = this.inFlight.get(key);
    if (pending) {
      return pending;
    }

    const task = (async () => {
      try {
        const bytes = await loader();
        return this.put(key, bytes, mimeType);
      } finally {
        this.inFlight.delete(key);
      }
    })();

    this.inFlight.set(key, task);
    return task;
  }

  revoke(key: string): boolean {
    return this.evictKey(key);
  }

  clear(): void {
    for (const entry of this.entries.values()) {
      this.revokeUrl(entry.url);
    }
    this.entries.clear();
    this.inFlight.clear();
    this.totalBytes = 0;
  }

  private evictKey(key: string): boolean {
    const entry = this.entries.get(key);
    if (!entry) return false;
    this.entries.delete(key);
    this.totalBytes = Math.max(0, this.totalBytes - entry.byteSize);
    this.revokeUrl(entry.url);
    return true;
  }
}

export const mediaBlobUrlCache = new MediaBlobUrlLruCache();
