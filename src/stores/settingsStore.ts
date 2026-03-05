import { create } from "zustand";
import type { AppConfig } from "../types/config";

interface SettingsState {
  config: AppConfig | null;
  setConfig: (config: AppConfig) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  config: null,
  setConfig: (config) => set({ config }),
}));
