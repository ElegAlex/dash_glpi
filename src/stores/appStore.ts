import { create } from "zustand";

interface AppState {
  currentImportId: number | null;
  dateRange: { from: string | null; to: string | null };
  setCurrentImportId: (id: number | null) => void;
  setDateRange: (from: string | null, to: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentImportId: null,
  dateRange: { from: null, to: null },
  setCurrentImportId: (id) => set({ currentImportId: id }),
  setDateRange: (from, to) => set({ dateRange: { from, to } }),
}));
