export interface StockOverview {
  totalVivants: number;
  totalTermines: number;
  parStatut: StatutCount[];
  ageMoyenJours: number;
  ageMedianJours: number;
  parType: TypeBreakdown;
  parAnciennete: AgeRangeCount[];
  inactifs14j: number;
  inactifs30j: number;
  nonAssignes: number;
}

export interface StatutCount {
  statut: string;
  count: number;
  estVivant: boolean;
}

export interface TypeBreakdown {
  incidents: number;
  demandes: number;
}

export interface AgeRangeCount {
  label: string;
  thresholdDays: number;
  count: number;
  percentage: number;
}

export interface TechnicianStats {
  technicien: string;
  total: number;
  enCours: number;
  enAttente: number;
  planifie: number;
  nouveau: number;
  incidents: number;
  demandes: number;
  ageMoyenJours: number;
  inactifs14j: number;
  ecartSeuil: number;
  couleurSeuil: string;
}

export interface BilanTemporel {
  periodes: PeriodData[];
  totaux: BilanTotaux;
  ventilation: BilanVentilation[] | null;
  resolution: BilanResolution | null;
}

export interface BilanResolution {
  tranches: ResolutionTranche[];
  totalResolus: number;
  mttrJours: number;
  medianeJours: number;
}

export interface ResolutionTranche {
  label: string;
  count: number;
  pourcentage: number;
}

export interface PeriodData {
  periodKey: string;
  periodLabel: string;
  entrees: number;
  sorties: number;
  delta: number;
  stockCumule: number | null;
}

export interface BilanTotaux {
  totalEntrees: number;
  totalSorties: number;
  deltaGlobal: number;
  moyenneEntreesParPeriode: number;
  moyenneSortiesParPeriode: number;
}

export interface BilanVentilation {
  label: string;
  entrees: number;
  sorties: number;
  delta: number;
}

export interface CategoryNode {
  name: string;
  fullPath: string;
  level: number;
  count: number;
  percentage: number;
  incidents: number;
  demandes: number;
  ageMoyen: number;
  children: CategoryNode[];
}

export interface CategoryTree {
  source: string;
  nodes: CategoryNode[];
  totalTickets: number;
}

export interface TicketSummary {
  id: number;
  titre: string;
  statut: string;
  typeTicket: string;
  technicienPrincipal: string | null;
  groupePrincipal: string | null;
  dateOuverture: string;
  derniereModification: string | null;
  ancienneteJours: number | null;
  inactiviteJours: number | null;
  nombreSuivis: number | null;
  actionRecommandee: string | null;
  motifClassification: string | null;
}
