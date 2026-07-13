import { invoke } from "@tauri-apps/api/core";
import type { Business, ContentBatch, ModelInfo, OllamaStatus, Preset, WorkspaceSettings } from "./types";

export const api = {
  getSettings: () => invoke<WorkspaceSettings>("get_settings"),
  saveSettings: (patch: Partial<WorkspaceSettings> & { byoApiKey?: string; unsplashApiKey?: string }) =>
    invoke<WorkspaceSettings>("save_settings", { patch }),
  getAvailableModels: () => invoke<ModelInfo[]>("get_available_models"),
  getOllamaStatus: () => invoke<OllamaStatus>("get_ollama_status"),
  pullOllamaModel: (modelId: string) =>
    invoke<{ ok: boolean; message: string }>("pull_ollama_model", { modelId }),
  connectNotion: (integrationToken: string) =>
    invoke<{ ok: boolean; message: string }>("connect_notion", { integrationToken }),
  testNotionConnection: () => invoke<{ ok: boolean; message: string }>("test_notion_connection"),
  testAiConnection: () => invoke<{ ok: boolean; message: string }>("test_ai_connection"),
  getPresets: () => invoke<Preset[]>("get_presets"),
  generateContent: (prompt: string) => invoke<ContentBatch>("generate_content", { prompt }),
  runPreset: (presetId: string, fieldValues: Record<string, string>) =>
    invoke<ContentBatch>("run_preset", { presetId, fieldValues }),
  discardNotionPush: (batchId: string) =>
    invoke<{ ok: boolean; message: string }>("discard_notion_push", { batchId }),
  listBatches: () => invoke<ContentBatch[]>("list_batches"),
  listBusinesses: () => invoke<Business[]>("list_businesses"),
  editBusiness: (businessId: string, prompt: string) =>
    invoke<ContentBatch>("edit_business", { businessId, prompt }),
};