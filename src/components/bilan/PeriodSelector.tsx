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
  week: 'Semaine',
  month: 'Mois',
  quarter: 'Trimestre',
};

const GRANULARITIES: Granularity[] = ['week', 'month', 'quarter'];

export function PeriodSelector({
  range,
  granularity,
  onRangeChange,
  onGranularityChange,
}: PeriodSelectorProps) {
  const autoGran = detectGranularity(range.from, range.to);

  return (
    <div className="rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)] p-4">
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between flex-wrap gap-2">
          <h3 className="text-sm font-semibold text-[#1a1f2e]">Période d'analyse</h3>
          <div className="flex items-center gap-2">
            <span className="text-xs text-[#6e7891]">Granularité :</span>
            <div className="flex rounded-md border border-[#cdd3df] overflow-hidden">
              {GRANULARITIES.map((g) => (
                <button
                  key={g}
                  onClick={() => onGranularityChange(g)}
                  className={`px-3 py-1 text-xs font-medium transition-colors border-r border-[#cdd3df] last:border-r-0 ${
                    granularity === g
                      ? 'bg-[#0C419A] text-white'
                      : 'bg-white text-[#525d73] hover:bg-[#f1f3f7]'
                  }`}
                >
                  {GRANULARITY_LABELS[g]}
                </button>
              ))}
            </div>
          </div>
        </div>

        <DateRangePicker value={range} onChange={onRangeChange} />

        <div className="text-xs text-[#6e7891]">
          Granularité auto-détectée :{' '}
          <span className="font-medium text-[#1a1f2e]">{GRANULARITY_LABELS[autoGran]}</span>
          {' — '}du{' '}
          <span className="font-medium text-[#1a1f2e]">
            {range.from.toLocaleDateString('fr-FR')}
          </span>{' '}
          au{' '}
          <span className="font-medium text-[#1a1f2e]">
            {range.to.toLocaleDateString('fr-FR')}
          </span>
        </div>
      </div>
    </div>
  );
}
