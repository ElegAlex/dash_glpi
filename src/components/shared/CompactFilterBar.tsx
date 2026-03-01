import {
  DateRangePicker,
  type DateRange,
  type Granularity,
} from './DateRangePicker';

interface CompactFilterBarProps {
  range: DateRange;
  granularity: Granularity;
  onRangeChange: (range: DateRange, granularity: Granularity) => void;
  onGranularityChange: (granularity: Granularity) => void;
  hideGranularity?: boolean;
}

const GRANULARITY_LABELS: Record<Granularity, string> = {
  day: 'Jour',
  week: 'Sem.',
  month: 'Mois',
  quarter: 'Trim.',
};

const GRANULARITIES: Granularity[] = ['day', 'week', 'month', 'quarter'];

export function CompactFilterBar({
  range,
  granularity,
  onRangeChange,
  onGranularityChange,
  hideGranularity,
}: CompactFilterBarProps) {
  return (
    <div className="flex items-center gap-3 flex-wrap mt-2">
      {/* Period presets + date inputs */}
      <DateRangePicker value={range} onChange={onRangeChange} />

      {!hideGranularity && (
        <>
          {/* Separator */}
          <div className="h-5 border-l border-slate-200/60" />

          {/* Granularity toggle */}
          <div className="flex items-center gap-1.5">
            <span className="text-[11px] text-slate-400 font-[Source_Sans_3]">Granularite</span>
            <div className="flex rounded-lg overflow-hidden bg-slate-100">
              {GRANULARITIES.map((g) => (
                <button
                  key={g}
                  onClick={() => onGranularityChange(g)}
                  className={`px-2.5 py-1 text-xs font-medium font-[DM_Sans] transition-colors ${
                    granularity === g
                      ? 'bg-[#0C419A] text-white'
                      : 'text-slate-500 hover:text-slate-700'
                  }`}
                >
                  {GRANULARITY_LABELS[g]}
                </button>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
