import { useState, useCallback } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  ImportEvent,
  ProgressData,
  WarningData,
  ImportResult,
} from "../types/tickets";

interface ImportState {
  isImporting: boolean;
  progress: number;
  phase: ProgressData["phase"] | null;
  result: ImportResult | null;
  error: string | null;
  warnings: WarningData[];
}

const INITIAL_STATE: ImportState = {
  isImporting: false,
  progress: 0,
  phase: null,
  result: null,
  error: null,
  warnings: [],
};

export function useImport() {
  const [state, setState] = useState<ImportState>(INITIAL_STATE);

  const startImport = useCallback(async (path: string) => {
    setState({ ...INITIAL_STATE, isImporting: true });

    const onProgress = new Channel<ImportEvent>();
    onProgress.onmessage = (event: ImportEvent) => {
      if (event.event === "progress") {
        const data = event.data as ProgressData;
        const pct =
          data.totalEstimated > 0
            ? Math.min(99, Math.round((data.rowsParsed / data.totalEstimated) * 100))
            : 0;
        setState((prev) => ({ ...prev, progress: pct, phase: data.phase }));
      } else if (event.event === "warning") {
        const data = event.data as WarningData;
        setState((prev) => ({ ...prev, warnings: [...prev.warnings, data] }));
      } else if (event.event === "complete") {
        setState((prev) => ({ ...prev, progress: 100 }));
      }
    };

    try {
      const result = await invoke<ImportResult>("import_csv", { path, onProgress });
      setState((prev) => ({
        ...prev,
        isImporting: false,
        progress: 100,
        result,
        warnings: result.warnings.length > 0 ? result.warnings : prev.warnings,
      }));
    } catch (err) {
      setState((prev) => ({
        ...prev,
        isImporting: false,
        error: err instanceof Error ? err.message : String(err),
      }));
    }
  }, []);

  const reset = useCallback(() => setState(INITIAL_STATE), []);

  return { ...state, startImport, reset };
}
