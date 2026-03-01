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

export interface TechTimelinePoint {
  importId: number;
  importDate: string;
  ticketCount: number;
  avgAge: number;
}

export interface TimelinePoint {
  importId: number;
  filename: string;
  importDate: string;
  vivantsCount: number;
  terminesCount: number;
  totalRows: number;
}

export interface TechnicianDelta {
  technicien: string;
  countA: number;
  countB: number;
  delta: number;
}

export interface ImportComparison {
  importA: ImportHistory;
  importB: ImportHistory;
  deltaTotal: number;
  deltaVivants: number;
  nouveauxTickets: number[];
  disparusTickets: number[];
  deltaParTechnicien: TechnicianDelta[];
}
