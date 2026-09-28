import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { LocalUsage, Range, Tool, ToolRoots } from "./types";

export const localUsage = (range: Range) => invoke<LocalUsage>("local_usage", { range });
export const rescan = () => invoke<LocalUsage>("rescan");
export const usageRoots = () => invoke<ToolRoots[]>("usage_roots");
export const setUsageRoots = (tool: Tool, paths: string[]) =>
  invoke<ToolRoots[]>("set_usage_roots", { tool, paths });
export const resetUsageRoots = () => invoke<ToolRoots[]>("reset_usage_roots");
export const setScale = (scale: number) => invoke<void>("set_scale", { scale });
export const scaleOptions = () => invoke<number[]>("scale_options");
export const openLogDir = () => invoke<void>("open_log_dir");
export const beginDialog = () => invoke<void>("begin_dialog");
export const endDialog = () => invoke<void>("end_dialog");
export const onUsageUpdated = (handler: () => void): Promise<UnlistenFn> =>
  listen("usage://updated", handler);
