import { create } from 'zustand';

interface FilterState {
  statut: string | null;
  typeTicket: string | null;
  groupe: string | null;
  resetFilters: () => void;
  setStatut: (v: string | null) => void;
  setTypeTicket: (v: string | null) => void;
  setGroupe: (v: string | null) => void;
}

export const useFilterStore = create<FilterState>((set) => ({
  statut: null,
  typeTicket: null,
  groupe: null,
  resetFilters: () => set({ statut: null, typeTicket: null, groupe: null }),
  setStatut: (v) => set({ statut: v }),
  setTypeTicket: (v) => set({ typeTicket: v }),
  setGroupe: (v) => set({ groupe: v }),
}));
