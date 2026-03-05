export interface AppConfig {
  seuilTicketsTechnicien: number;
  seuilAncienneteCloture: number;
  seuilInactiviteCloture: number;
  seuilAncienneteRelancer: number;
  seuilInactiviteRelancer: number;
  seuilCouleurVert: number;
  seuilCouleurJaune: number;
  seuilCouleurOrange: number;
  seuilSimilariteDoublons: number;
  statutsVivants: string[];
  statutsTermines: string[];
  taillePoliceAxes: number;
}

export interface ImportHistory {
  id: number;
  filename: string;
  importDate: string;
  totalRows: number;
  vivantsCount: number;
  terminesCount: number;
  dateRangeFrom: string | null;
  dateRangeTo: string | null;
  isActive: boolean;
}

export interface ExportResult {
  path: string;
  sizeBytes: number;
  durationMs: number;
}

export interface TechHistory {
  kpi: TechHistoryKpi;
  periodes: TechHistoryPeriod[];
}

export interface TechHistoryKpi {
  totalEntrants: number;
  totalSortants: number;
  stockActuel: number;
  mttrJours: number | null;
  incidents: number;
  demandes: number;
  ageMoyenJours: number;
}

export interface TechHistoryPeriod {
  periodKey: string;
  entrants: number;
  sortants: number;
  stockCumule: number;
  mttrJours: number | null;
}
