export interface ImportEvent {
  event: "progress" | "complete" | "warning";
  data: ProgressData | CompleteData | WarningData;
}

export interface ProgressData {
  rowsParsed: number;
  totalEstimated: number;
  phase: "parsing" | "normalizing" | "inserting" | "indexing";
}

export interface CompleteData {
  durationMs: number;
  totalTickets: number;
  vivants: number;
  termines: number;
}

export interface WarningData {
  line: number;
  message: string;
}

export interface ImportResult {
  importId: number;
  totalTickets: number;
  vivantsCount: number;
  terminesCount: number;
  skippedRows: number;
  warnings: ParseWarning[];
  detectedColumns: string[];
  missingOptionalColumns: string[];
  uniqueStatuts: string[];
  parseDurationMs: number;
}

export interface ParseWarning {
  line: number;
  message: string;
}

export interface GLPITicket {
  id: number;
  titre: string;
  statut: string;
  typeTicket: string;
  priorite: number | null;
  urgence: number | null;
  demandeur: string;
  technicienPrincipal: string | null;
  groupePrincipal: string | null;
  dateOuverture: string;
  derniereModification: string | null;
  nombreSuivis: number | null;
  ancienneteJours: number | null;
  inactiviteJours: number | null;
  actionRecommandee: string | null;
  motifClassification: string | null;
}
