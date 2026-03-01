import { useState, useEffect } from 'react';
import { subDays, startOfQuarter, endOfQuarter, subQuarters, differenceInDays, format, parse, isValid } from 'date-fns';

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

type Preset = '7d' | '30d' | 'quarter';

interface DateRangePickerProps {
  value: DateRange;
  onChange: (range: DateRange, granularity: Granularity) => void;
}

const PRESETS: { id: Preset; label: string }[] = [
  { id: '7d', label: '7j' },
  { id: '30d', label: '30j' },
  { id: 'quarter', label: 'Trim.' },
];

function getPresetRange(preset: Preset): DateRange {
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
  }
}

function formatFr(d: Date): string {
  return format(d, 'dd/MM/yyyy');
}

function parseFr(s: string): Date | null {
  const d = parse(s.trim(), 'dd/MM/yyyy', new Date());
  return isValid(d) ? d : null;
}

function DateInput({
  value,
  onChange,
}: {
  value: Date;
  onChange: (d: Date) => void;
}) {
  const [text, setText] = useState(formatFr(value));
  const [invalid, setInvalid] = useState(false);

  // Sync text when value changes externally (presets)
  useEffect(() => {
    setText(formatFr(value));
    setInvalid(false);
  }, [value]);

  const commit = () => {
    const d = parseFr(text);
    if (d) {
      setInvalid(false);
      onChange(d);
    } else {
      setInvalid(true);
      // Reset to current valid value after a brief flash
      setTimeout(() => {
        setText(formatFr(value));
        setInvalid(false);
      }, 800);
    }
  };

  return (
    <input
      type="text"
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          e.currentTarget.blur();
        }
      }}
      placeholder="jj/mm/aaaa"
      className={`w-[88px] px-2 py-1 rounded-lg text-xs font-medium font-[DM_Sans] text-center
        bg-slate-100 border outline-none transition-colors
        ${invalid
          ? 'border-red-400 text-red-600'
          : 'border-transparent focus:border-[#0C419A]/30 text-slate-700'
        }`}
    />
  );
}

export function DateRangePicker({ value, onChange }: DateRangePickerProps) {
  const handlePreset = (preset: Preset) => {
    const range = getPresetRange(preset);
    onChange(range, detectGranularity(range.from, range.to));
  };

  const handleFromChange = (d: Date) => {
    const range = { from: d, to: value.to < d ? d : value.to };
    onChange(range, detectGranularity(range.from, range.to));
  };

  const handleToChange = (d: Date) => {
    const range = { from: value.from > d ? d : value.from, to: d };
    onChange(range, detectGranularity(range.from, range.to));
  };

  return (
    <div className="flex items-center gap-1.5">
      {PRESETS.map((p) => (
        <button
          key={p.id}
          onClick={() => handlePreset(p.id)}
          className="px-2.5 py-1 rounded-lg text-xs font-medium font-[DM_Sans] transition-colors text-slate-500 hover:bg-slate-100 hover:text-slate-700"
        >
          {p.label}
        </button>
      ))}

      <span className="text-[11px] text-slate-400 font-[Source_Sans_3] ml-1">du</span>
      <DateInput value={value.from} onChange={handleFromChange} />
      <span className="text-[11px] text-slate-400 font-[Source_Sans_3]">au</span>
      <DateInput value={value.to} onChange={handleToChange} />
    </div>
  );
}
