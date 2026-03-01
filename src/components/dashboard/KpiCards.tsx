import { Clock, Timer, UserCheck, TrendingUp, AlertTriangle } from 'lucide-react';
import { KpiCard } from '../shared/KpiCard';
import type { DashboardKpi } from '../../types/dashboard';

interface KpiCardsProps {
  kpi: DashboardKpi;
}

export function KpiCards({ kpi }: KpiCardsProps) {
  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-5">
      <KpiCard
        label="Delai resolution (MTTR)"
        value={Number(kpi.resolution.mttrGlobalJours.toFixed(1))}
        format="days"
        icon={<Clock size={18} color="#1565C0" />}
        accentColor="#1565C0"
      />
      <KpiCard
        label="Mediane resolution"
        value={Number(kpi.resolution.medianeJours.toFixed(1))}
        format="days"
        icon={<Timer size={18} color="#6A1B9A" />}
        accentColor="#6A1B9A"
      />
      <KpiCard
        label="Taux N1"
        value={Number(kpi.tauxN1.n1Elargi.pourcentage.toFixed(1))}
        format="percent"
        icon={<UserCheck size={18} color="#2E7D32" />}
        accentColor="#2E7D32"
      />
      <KpiCard
        label="Creations / mois"
        value={Math.round(kpi.volumes.moyenneMensuelleCreation)}
        format="number"
        icon={<TrendingUp size={18} color="#FF8F00" />}
        accentColor="#FF8F00"
      />
      <KpiCard
        label="Stock ouvert"
        value={kpi.meta.totalVivants}
        format="number"
        icon={<AlertTriangle size={18} color="#C62828" />}
        accentColor="#C62828"
      />
    </div>
  );
}
