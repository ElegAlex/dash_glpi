import type { ReactNode } from 'react';

interface KpiCardProps {
  label: string;
  value: number | string;
  previousValue?: number;
  format?: 'number' | 'percent' | 'days';
  trend?: 'up' | 'down' | 'neutral';
  trendIsGood?: boolean;
  icon?: ReactNode;
  accentColor?: string;
}

export function KpiCard({
  label, value, previousValue, format = 'number',
  trend, trendIsGood = true, icon, accentColor = '#0C419A',
}: KpiCardProps) {
  const displayValue = format === 'percent'
    ? `${value}%`
    : format === 'days'
      ? `${value}j`
      : typeof value === 'number'
        ? value.toLocaleString('fr-FR')
        : value;

  const delta = previousValue != null && typeof value === 'number'
    ? ((value - previousValue) / previousValue * 100).toFixed(1)
    : null;

  const isGood = trend === 'neutral' ? null
    : trend === 'up' ? trendIsGood
    : !trendIsGood;

  return (
    <div className="
      relative overflow-hidden bg-white rounded-2xl p-5
      shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
      hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]
      hover:-translate-y-0.5
      transition-all duration-200 ease-[cubic-bezier(0.4,0,0.2,1)]
      group
    ">
      {/* Barre d'accent top */}
      <div
        className="absolute top-0 inset-x-0 h-[3px] rounded-t-2xl"
        style={{ background: `linear-gradient(90deg, ${accentColor}, ${accentColor}88)` }}
      />

      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <span className="text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
          {label}
        </span>
        {icon && (
          <div
            className="w-9 h-9 rounded-[10px] flex items-center justify-center"
            style={{ background: `${accentColor}12` }}
          >
            {icon}
          </div>
        )}
      </div>

      {/* Valeur + tendance */}
      <div className="flex items-baseline gap-2.5">
        <span className="text-[36px] font-bold font-[DM_Sans] tracking-tight leading-none text-slate-800">
          {displayValue}
        </span>
        {delta != null && trend && trend !== 'neutral' && (
          <span className={`
            text-xs font-semibold px-2 py-0.5 rounded-lg flex items-center gap-0.5
            ${isGood ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'}
          `}>
            {trend === 'up' ? '\u2191' : '\u2193'} {delta}%
          </span>
        )}
      </div>

      {/* Barre de progression subtile */}
      <div className="mt-4 h-[3px] rounded-full bg-slate-100 overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-700 ease-out"
          style={{
            background: `linear-gradient(90deg, ${accentColor}, ${accentColor}AA)`,
            width: `${Math.min(100, typeof value === 'number' ? (value / 10000) * 100 : 50)}%`,
          }}
        />
      </div>
    </div>
  );
}
