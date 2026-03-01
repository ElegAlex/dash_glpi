import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface UseInvokeResult<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  execute: (...args: Parameters<typeof invoke>) => Promise<T | null>;
}

export function useInvoke<T>(): UseInvokeResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(
    async (...args: Parameters<typeof invoke>): Promise<T | null> => {
      setLoading(true);
      setError(null);
      try {
        const result = (await invoke(...args)) as T;
        setData(result);
        return result;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        return null;
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  return { data, loading, error, execute };
}
