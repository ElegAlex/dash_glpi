import { useState } from 'react';
import { DayPicker, type DateRange as DayPickerRange } from 'react-day-picker';
import { fr } from 'react-day-picker/locale';
import { subDays, startOfQuarter, endOfQuarter, subQuarters, differenceInDays } from 'date-fns';
import 'react-day-picker/style.css';

export type Granularity = 'day' | 'week' | 'month' | 'quarter';

export interface DateRange {
  from: Date;
  to: Date;
}

export function detectGranularity(from: Date, to: Date): Granularity {
  const days = differenceInDays(to, from);
  if (days <= 14) return 'day';
  if (days < 30) return 'week';
  if (days < 365) return 'month';
  return 'quarter';
}

type Preset = '7d' | '30d' | 'quarter' | 'custom';

interface DateRangePickerProps {
  value: DateRange;
  onChange: (range: DateRange, granularity: Granularity) => void;
}

const PRESETS: { id: Preset; label: string }[] = [
  { id: '7d', label: '7 derniers jours' },
  { id: '30d', label: '30 derniers jours' },
  { id: 'quarter', label: 'Dernier trimestre' },
  { id: 'custom', label: 'Personnalisé' },
];

function getPresetRange(preset: Preset): DateRange | null {
  const today = new Date();
  switch (preset) {
    case '7d':
      return { from: subDays(today, 6), to: today };
    case '30d':
      return { from: subDays(today, 29), to: today };
    case 'quarter': {
      const lastQ = subQuarters(today, 1);
      return { from: startOfQuarter(lastQ), to: endOfQuarter(lastQ) };
    }
    case 'custom':
      return null;
  }
}

export function DateRangePicker({ value, onChange }: DateRangePickerProps) {
  const [activePreset, setActivePreset] = useState<Preset>('30d');
  const [showCalendar, setShowCalendar] = useState(false);
  const [pendingRange, setPendingRange] = useState<DayPickerRange | undefined>({
    from: value.from,
    to: value.to,
  });

  const handlePreset = (preset: Preset) => {
    setActivePreset(preset);
    if (preset === 'custom') {
      setShowCalendar(true);
      return;
    }
    setShowCalendar(false);
    const range = getPresetRange(preset);
    if (range) {
      onChange(range, detectGranularity(range.from, range.to));
    }
  };

  const handleRangeSelect = (range: DayPickerRange | undefined) => {
    setPendingRange(range);
    if (range?.from && range?.to) {
      onChange({ from: range.from, to: range.to }, detectGranularity(range.from, range.to));
    }
  };

  return (
    <div className="flex gap-3 items-start flex-wrap">
      <div className="flex flex-col gap-1 min-w-[160px]">
        {PRESETS.map((p) => (
          <button
            key={p.id}
            onClick={() => handlePreset(p.id)}
            className={`text-left px-3 py-2 rounded-xl text-sm font-medium transition-colors ${
              activePreset === p.id
                ? 'bg-primary-500 text-white'
                : 'text-slate-500 hover:bg-[rgba(12,65,154,0.04)] hover:text-slate-800'
            }`}
          >
            {p.label}
          </button>
        ))}
      </div>

      {showCalendar && (
        <div className="rounded-2xl bg-white shadow-[0_10px_20px_rgba(0,0,0,0.10),0_3px_6px_rgba(0,0,0,0.06)] p-3">
          <DayPicker
            mode="range"
            locale={fr}
            selected={pendingRange}
            onSelect={handleRangeSelect}
            numberOfMonths={2}
          />
        </div>
      )}
    </div>
  );
}
