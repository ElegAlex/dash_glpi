export interface DelaisKpi {
  taux24h: number;
  taux48h: number;
  mttrJours: number;
  medianeJours: number;
  totalResolus: number;
  trend: DelaisTrend[];
  distribution: TrancheDelai[];
}

export interface DelaisTrend {
  periodKey: string;
  periodLabel: string;
  pct24h: number;
  pct48h: number;
  totalResolus: number;
}

export interface TrancheDelai {
  label: string;
  count: number;
  pourcentage: number;
}

export interface CategorieDelais {
  categorie: string;
  totalResolus: number;
  mttrJours: number;
  medianeJours: number;
  taux24h: number;
  taux48h: number;
}

export interface CategoriesDelaisRequest {
  dateFrom: string;
  dateTo: string;
  column: string;
  parentColumn?: string;
  parentValue?: string;
}

export interface DelaisParCategorieRequest {
  dateFrom: string;
  dateTo: string;
  categorieNiveau1?: string;
  categorieNiveau2?: string;
  categorie?: string;
}
