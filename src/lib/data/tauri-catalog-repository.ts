import { invoke } from "@tauri-apps/api/core";
import {
  encodeDocumentIpcFrame,
  encodePhotoIpcFrame,
} from "$lib/utils/media-ipc-frame";
import type {
  AttachModelDocumentInput,
  AttachModelPhotoInput,
  AttributeDefinition,
  Brand,
  CatalogListFilter,
  CatalogListResult,
  CatalogRepository,
  CreateAttributeDefinitionInput,
  CreateBrandInput,
  CreateCategoryInput,
  CreateEquipmentModelInput,
  EquipmentCategory,
  EquipmentCategoryTreeItem,
  EquipmentModelMediaView,
  EquipmentModelWithDetails,
  InheritedAttributeDefinition,
  MediaGarbageCollectionResult,
  ReorderModelMediaInput,
  SuggestedCategoriesResult,
  UpdateAttributeDefinitionInput,
  UpdateBrandInput,
  UpdateCategoryInput,
  UpdateEquipmentModelInput,
  UpdateModelMediaInput,
} from "./catalog-repository";

export class TauriCatalogRepository implements CatalogRepository {
  async listBrands(includeInactive = false): Promise<Brand[]> {
    return invoke<Brand[]>("list_brands", { includeInactive });
  }

  async createBrand(input: CreateBrandInput): Promise<Brand> {
    return invoke<Brand>("create_brand", { input });
  }

  async updateBrand(id: string, input: UpdateBrandInput): Promise<Brand> {
    return invoke<Brand>("update_brand", { id, input });
  }

  async deleteBrand(id: string): Promise<void> {
    return invoke<void>("delete_brand", { id });
  }

  async listCategoriesTree(
    includeInactive = false,
  ): Promise<EquipmentCategoryTreeItem[]> {
    return invoke<EquipmentCategoryTreeItem[]>("list_categories_tree", {
      includeInactive,
    });
  }

  async getCategoryEffectiveAttributes(
    categoryId: string,
  ): Promise<InheritedAttributeDefinition[]> {
    return invoke<InheritedAttributeDefinition[]>(
      "get_category_effective_attributes",
      {
        categoryId,
      },
    );
  }

  async createCategory(input: CreateCategoryInput): Promise<EquipmentCategory> {
    return invoke<EquipmentCategory>("create_category", { input });
  }

  async updateCategory(
    id: string,
    input: UpdateCategoryInput,
  ): Promise<EquipmentCategory> {
    return invoke<EquipmentCategory>("update_category", { id, input });
  }

  async deleteCategory(id: string): Promise<void> {
    return invoke<void>("delete_category", { id });
  }

  async createAttributeDefinition(
    input: CreateAttributeDefinitionInput,
  ): Promise<AttributeDefinition> {
    return invoke<AttributeDefinition>("create_attribute_definition", {
      input,
    });
  }

  async updateAttributeDefinition(
    id: string,
    input: UpdateAttributeDefinitionInput,
  ): Promise<AttributeDefinition> {
    return invoke<AttributeDefinition>("update_attribute_definition", {
      id,
      input,
    });
  }

  async deleteAttributeDefinition(id: string): Promise<void> {
    return invoke<void>("delete_attribute_definition", { id });
  }

  async listEquipmentModels(
    filter: CatalogListFilter,
  ): Promise<CatalogListResult> {
    return invoke<CatalogListResult>("list_equipment_models", { filter });
  }

  async getEquipmentModel(id: string): Promise<EquipmentModelWithDetails> {
    return invoke<EquipmentModelWithDetails>("get_equipment_model", { id });
  }

  async createEquipmentModel(
    input: CreateEquipmentModelInput,
  ): Promise<EquipmentModelWithDetails> {
    return invoke<EquipmentModelWithDetails>("create_equipment_model", {
      input,
    });
  }

  async updateEquipmentModel(
    id: string,
    input: UpdateEquipmentModelInput,
  ): Promise<EquipmentModelWithDetails> {
    return invoke<EquipmentModelWithDetails>("update_equipment_model", {
      id,
      input,
    });
  }

  async setEquipmentModelActive(id: string, active: boolean): Promise<void> {
    return invoke<void>("set_equipment_model_active", { id, active });
  }

  async deleteEquipmentModel(id: string): Promise<void> {
    return invoke<void>("delete_equipment_model", { id });
  }

  async loadSuggestedCategories(): Promise<SuggestedCategoriesResult> {
    return invoke<SuggestedCategoriesResult>(
      "load_suggested_catalog_categories",
    );
  }

  async listModelMedia(modelId: string): Promise<EquipmentModelMediaView[]> {
    return invoke<EquipmentModelMediaView[]>("list_model_media", { modelId });
  }

  async getMediaBlobBytes(blobId: string): Promise<Uint8Array> {
    const raw = await invoke<ArrayBuffer | Uint8Array | number[]>(
      "get_media_blob_bytes",
      { blobId },
    );
    if (raw instanceof Uint8Array) {
      return raw;
    }
    if (raw instanceof ArrayBuffer) {
      return new Uint8Array(raw);
    }
    return new Uint8Array(raw);
  }

  async attachModelPhoto(
    input: AttachModelPhotoInput,
  ): Promise<EquipmentModelMediaView> {
    const frame = encodePhotoIpcFrame(input);
    return invoke<EquipmentModelMediaView>("attach_model_photo", frame);
  }

  async attachModelDocument(
    input: AttachModelDocumentInput,
  ): Promise<EquipmentModelMediaView> {
    const frame = encodeDocumentIpcFrame(input);
    return invoke<EquipmentModelMediaView>("attach_model_document", frame);
  }

  async updateModelMediaMetadata(
    input: UpdateModelMediaInput,
  ): Promise<EquipmentModelMediaView> {
    return invoke<EquipmentModelMediaView>("update_model_media_metadata", {
      input,
    });
  }

  async reorderModelMedia(
    input: ReorderModelMediaInput,
  ): Promise<EquipmentModelMediaView[]> {
    return invoke<EquipmentModelMediaView[]>("reorder_model_media", { input });
  }

  async setPrimaryModelPhoto(
    mediaLinkId: string,
  ): Promise<EquipmentModelMediaView> {
    return invoke<EquipmentModelMediaView>("set_primary_model_photo", {
      mediaLinkId,
    });
  }

  async removeModelMedia(mediaLinkId: string): Promise<void> {
    return invoke<void>("remove_model_media", { mediaLinkId });
  }

  async collectUnreferencedMedia(): Promise<MediaGarbageCollectionResult> {
    return invoke<MediaGarbageCollectionResult>("collect_unreferenced_media");
  }

  async openModelDocumentExternal(mediaLinkId: string): Promise<string> {
    return invoke<string>("open_model_document_external", { mediaLinkId });
  }

  async saveModelDocumentAs(mediaLinkId: string): Promise<string | null> {
    return invoke<string | null>("save_model_document_as", { mediaLinkId });
  }
}
