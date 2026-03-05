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
