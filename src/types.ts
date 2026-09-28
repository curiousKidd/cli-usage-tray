export type Range = "today" | "week" | "all";
export type Tool = "claude-code" | "codex" | "gemini-cli";

export interface Roots {
  custom: Partial<Record<Tool, string[]>>;
}

export interface UsageRecord {
  key: string;
  occurredAt: string;
  inputTokens: number;
  outputTokens: number;
  cacheWriteTokens: number;
  cacheReadTokens: number;
  model: string;
  source: Tool;
}

export interface Counter {
  inputTokens: number;
  outputTokens: number;
  cacheWriteTokens: number;
  cacheReadTokens: number;
  totalTokens: number;
  records: number;
}

export interface ToolUsage {
  tool: Tool;
  counter: Counter;
}

export interface ModelUsage {
  model: string;
  tool: Tool;
  counter: Counter;
}

export interface DailyUsage {
  date: string;
  totalTokens: number;
}

export interface LocalUsage {
  range: Range;
  generatedAt: string;
  lastScanAt: string | null;
  scanning: boolean;
  total: Counter;
  byTool: ToolUsage[];
  byModel: ModelUsage[];
  daily: DailyUsage[];
  warnings: string[];
}

export interface RootPath {
  path: string;
  exists: boolean;
  files: number;
}

export interface ToolRoots {
  tool: Tool;
  defaults: RootPath[];
  custom: RootPath[];
}
