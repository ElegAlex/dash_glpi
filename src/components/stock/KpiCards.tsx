import type { StockOverview } from '../../types/kpi';

type ThresholdLevel = 'green' | 'yellow' | 'orange' | 'red';

const THRESHOLD_STYLES: Record<ThresholdLevel, { border: string; text: string; bg: string }> = {
  green:  { border: 'border-l-[#18753c]', text: 'text-[#136130]', bg: 'bg-[#f0faf4]' },
  yellow: { border: 'border-l-[#b27806]', text: 'text-[#965e04]', bg: 'bg-[#fef9ec]' },
  orange: { border: 'border-l-[#d4600a]', text: 'text-[#d4600a]', bg: 'bg-orange-50' },
  red:    { border: 'border-l-[#ce0500]', text: 'text-[#af0400]', bg: 'bg-[#fef2f2]' },
};

function getLevel(
  value: number,
  thresholds: { green: number; yellow: number; orange: number },
): ThresholdLevel {
  if (value <= thresholds.green) return 'green';
  if (value <= thresholds.yellow) return 'yellow';
  if (value <= thresholds.orange) return 'orange';
  return 'red';
}

interface KpiCardProps {
  title: string;
  value: string;
  level: ThresholdLevel;
  sub?: string;
}

function KpiCard({ title, value, level, sub }: KpiCardProps) {
  const s = THRESHOLD_STYLES[level];
  return (
    <div className={`rounded-lg border border-[#e2e6ee] bg-white p-5 shadow-[0_1px_2px_0_rgb(26_31_46/0.04)] border-l-4 ${s.border} ${s.bg}`}>
      <p className="text-sm font-medium text-[#525d73]">{title}</p>
      <p className={`mt-2 text-3xl font-semibold ${s.text}`}>{value}</p>
      {sub && <p className="mt-1 text-xs text-[#6e7891]">{sub}</p>}
    </div>
  );
}

interface KpiCardsProps {
  overview: StockOverview;
}

export function KpiCards({ overview }: KpiCardsProps) {
  const stockLevel = getLevel(overview.totalVivants, { green: 500, yellow: 700, orange: 900 });
  const ageLevel = getLevel(overview.ageMedianJours, { green: 10, yellow: 20, orange: 40 });

  const over90 = overview.parAnciennete.find((r) => r.thresholdDays === 90)?.count ?? 0;
  const over90Level = getLevel(over90, { green: 10, yellow: 20, orange: 40 });

  const totalTyped = overview.parType.incidents + overview.parType.demandes;
  const incidentPct = totalTyped > 0 ? Math.round((overview.parType.incidents / totalTyped) * 100) : 0;

  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
      <KpiCard
        title="Stock vivant"
        value={overview.totalVivants.toLocaleString('fr-FR')}
        level={stockLevel}
        sub={`${overview.totalTermines.toLocaleString('fr-FR')} terminés`}
      />
      <KpiCard
        title="Âge médian"
        value={`${overview.ageMedianJours} j`}
        level={ageLevel}
        sub={`Moyenne : ${overview.ageMoyenJours} j`}
      />
      <KpiCard
        title="Tickets > 90 j"
        value={over90.toLocaleString('fr-FR')}
        level={over90Level}
        sub="Seuil critique RG-018"
      />
      <KpiCard
        title="Incidents / Demandes"
        value={`${overview.parType.incidents} / ${overview.parType.demandes}`}
        level="green"
        sub={`${incidentPct}% incidents`}
      />
    </div>
  );
}
