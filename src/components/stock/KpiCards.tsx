import { Activity, CheckCircle, Clock, AlertTriangle, UserX } from 'lucide-react';
import { KpiCard } from '../shared/KpiCard';
import type { StockOverview } from '../../types/kpi';

interface KpiCardsProps {
  overview: StockOverview;
  onUnassignedClick?: () => void;
}

export function KpiCards({ overview, onUnassignedClick }: KpiCardsProps) {
  const over90 = overview.parAnciennete.find((r) => r.thresholdDays === 90)?.count ?? 0;

  return (
    <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 xl:grid-cols-5">
      <KpiCard
        label="Stock vivant"
        value={overview.totalVivants}
        icon={<Activity size={18} className="text-primary-500" />}
        accentColor="#0C419A"
      />
      <KpiCard
        label="Termines"
        value={overview.totalTermines}
        icon={<CheckCircle size={18} className="text-emerald-600" />}
        accentColor="#2E7D32"
      />
      <KpiCard
        label="Non assignes"
        value={overview.nonAssignes}
        icon={<UserX size={18} className="text-orange-600" />}
        accentColor="#E65100"
        onClick={onUnassignedClick}
      />
      <KpiCard
        label="Age median"
        value={overview.ageMedianJours}
        format="days"
        icon={<Clock size={18} className="text-amber-600" />}
        accentColor="#FF8F00"
      />
      <KpiCard
        label="Tickets > 90 j"
        value={over90}
        icon={<AlertTriangle size={18} className="text-red-600" />}
        accentColor="#C62828"
      />
    </div>
  );
}
