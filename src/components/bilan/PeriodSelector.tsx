import {
  DateRangePicker,
  detectGranularity,
  type DateRange,
  type Granularity,
} from '../shared/DateRangePicker';

interface PeriodSelectorProps {
  range: DateRange;
  granularity: Granularity;
  onRangeChange: (range: DateRange, granularity: Granularity) => void;
  onGranularityChange: (granularity: Granularity) => void;
}

const GRANULARITY_LABELS: Record<Granularity, string> = {
  day: 'Jour',
  week: 'Semaine',
  month: 'Mois',
  quarter: 'Trimestre',
};

const GRANULARITIES: Granularity[] = ['day', 'week', 'month', 'quarter'];

export function PeriodSelector({
  range,
  granularity,
  onRangeChange,
  onGranularityChange,
}: PeriodSelectorProps) {
  const autoGran = detectGranularity(range.from, range.to);

  return (
    <div className="rounded-2xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-4">
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between flex-wrap gap-2">
          <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-800">Periode d'analyse</h3>
          <div className="flex items-center gap-2">
            <span className="text-xs text-slate-400">Granularite :</span>
            <div className="flex rounded-xl overflow-hidden bg-slate-100">
              {GRANULARITIES.map((g) => (
                <button
                  key={g}
                  onClick={() => onGranularityChange(g)}
                  className={`px-3 py-1 text-xs font-medium transition-colors ${
                    granularity === g
                      ? 'bg-primary-500 text-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]'
                      : 'text-slate-500 hover:text-slate-800'
                  }`}
                >
                  {GRANULARITY_LABELS[g]}
                </button>
              ))}
            </div>
          </div>
        </div>

        <DateRangePicker value={range} onChange={onRangeChange} />

        <div className="text-xs text-slate-400">
          Granularite auto-detectee :{' '}
          <span className="font-medium text-slate-800 font-[DM_Sans]">{GRANULARITY_LABELS[autoGran]}</span>
          {' — '}du{' '}
          <span className="font-medium text-slate-800 font-[DM_Sans]">
            {range.from.toLocaleDateString('fr-FR')}
          </span>{' '}
          au{' '}
          <span className="font-medium text-slate-800 font-[DM_Sans]">
            {range.to.toLocaleDateString('fr-FR')}
          </span>
        </div>
      </div>
    </div>
  );
}
