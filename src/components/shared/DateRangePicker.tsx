import { useState } from 'react';
import { DayPicker, type DateRange as DayPickerRange } from 'react-day-picker';
import { fr } from 'react-day-picker/locale';
import { subDays, startOfQuarter, endOfQuarter, subQuarters, differenceInDays } from 'date-fns';
import 'react-day-picker/style.css';

export type Granularity = 'week' | 'month' | 'quarter';

export interface DateRange {
  from: Date;
  to: Date;
}

export function detectGranularity(from: Date, to: Date): Granularity {
  const days = differenceInDays(to, from);
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
            className={`text-left px-3 py-2 rounded-md text-sm font-medium transition-colors ${
              activePreset === p.id
                ? 'bg-[#0C419A] text-white'
                : 'text-[#525d73] hover:bg-[#f1f3f7] hover:text-[#1a1f2e]'
            }`}
          >
            {p.label}
          </button>
        ))}
      </div>

      {showCalendar && (
        <div className="border border-[#e2e6ee] rounded-lg bg-white shadow-[0_4px_6px_-1px_rgb(26_31_46/0.06)] p-3">
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
