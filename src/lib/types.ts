export interface WorkspaceSettings {
  notionConnected: boolean;
  notionParentPageId: string | null;
  aiProvider: "local" | "byo";
  selectedModelId: string;
  byoModel: string;
  byoKeySet: boolean;
  unsplashKeySet: boolean;
}

export interface ModelInfo {
  id: string;
  label: string;
  source: "local";
  status: "ready" | "pull_required" | "insufficient_ram" | "not_installed" | "available";
  detail: string;
  recommended: boolean;
}

export interface OllamaStatus {
  installed: boolean;
  running: boolean;
  pulledModels: string[];
}

export interface Preset {
  id: string;
  label: string;
  promptTemplate: string;
  fields: string[];
}

export interface ContentItem {
  type: "headline" | "subline" | "quote" | "tip" | "calendar";
  /** Short hook/title. Present for headline/quote/tip, and mirrored onto calendar entries. */
  title?: string;
  /** Longer body/description text. */
  text: string;
  date?: string;
  platform?: string;
  /** Calendar entries only: which type of content this day-entry came from. */
  contentType?: "headline" | "quote" | "tip";
}

export interface Business {
  id: string;
  name: string;
  context: string;
  hubPageId: string;
  createdAt: string;
}

export interface ContentBatch {
  id: string;
  createdAt: string;
  status: "draft" | "pushed" | "archived";
  items: ContentItem[];
  /** The real reason the push to Notion failed, if it did. */
  pushError?: string;
}