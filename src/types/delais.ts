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
