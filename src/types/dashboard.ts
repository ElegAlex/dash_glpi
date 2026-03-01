export interface DashboardKpi {
  meta: DashboardMeta;
  priseEnCharge: PriseEnChargeKpi;
  resolution: ResolutionKpi;
  tauxN1: TauxN1Kpi;
  volumes: VolumetrieKpi;
  typologie: TypologieKpi;
}

export interface DashboardMeta {
  totalTickets: number;
  totalVivants: number;
  totalTermines: number;
  plageDates: [string, string];
  nbTechniciensActifs: number;
  nbGroupes: number;
  hasCategorie: boolean;
  calculDurationMs: number;
}

export interface PriseEnChargeKpi {
  methode: string;
  confiance: string;
  delaiMoyenJours: number | null;
  medianeJours: number | null;
  p90Jours: number | null;
  distribution: TrancheDelai[];
  avertissement: string | null;
}

export interface ResolutionKpi {
  mttrGlobalJours: number;
  medianeJours: number;
  p90Jours: number;
  ecartTypeJours: number;
  parType: MttrParDimension[];
  parPriorite: MttrParDimension[];
  parGroupe: MttrParDimension[];
  parTechnicien: MttrParDimension[];
  distributionTranches: TrancheDelai[];
  trendMensuel: MttrTrend[];
  echantillon: number;
}

export interface TauxN1Kpi {
  totalTermines: number;
  n1Strict: TauxDetail;
  n1Elargi: TauxDetail;
  multiNiveaux: TauxDetail;
  sansTechnicien: TauxDetail;
  parGroupe: TauxN1ParGroupe[];
  trendMensuel: TauxN1Trend[];
  objectifItil: number;
}

export interface VolumetrieKpi {
  parMois: VolumePeriode[];
  totalCrees: number;
  totalResolus: number;
  ratioSortieEntree: number;
  moyenneMensuelleCreation: number;
}

export interface TypologieKpi {
  parType: VentilationItem[];
  parPriorite: VentilationItem[];
  parGroupe: VentilationItem[];
  parCategorie: VentilationItem[] | null;
  categorieDisponible: boolean;
}

export interface TrancheDelai {
  label: string;
  count: number;
  pourcentage: number;
}

export interface MttrParDimension {
  label: string;
  mttrJours: number;
  medianeJours: number;
  count: number;
  pourcentageTotal: number;
}

export interface MttrTrend {
  periode: string;
  mttrJours: number;
  medianeJours: number;
  nbResolus: number;
}

export interface TauxDetail {
  count: number;
  pourcentage: number;
}

export interface TauxN1ParGroupe {
  groupe: string;
  totalResolus: number;
  n1StrictCount: number;
  n1StrictPct: number;
  n1ElargiCount: number;
  n1ElargiPct: number;
}

export interface TauxN1Trend {
  periode: string;
  n1StrictPct: number;
  n1ElargiPct: number;
  totalResolus: number;
}

export interface VolumePeriode {
  periode: string;
  crees: number;
  resolus: number;
  delta: number;
}

export interface VentilationItem {
  label: string;
  total: number;
  vivants: number;
  termines: number;
  pourcentageTotal: number;
}
