import type { ReactNode } from 'react';

interface BilanRow {
  label: string;
  value: number | string;
  highlight?: boolean;
  positive?: boolean;
  negative?: boolean;
  icon?: ReactNode;
}

export function BilanTable({ rows }: { rows: BilanRow[] }) {
  return (
    <div className="bg-white rounded-2xl overflow-hidden
      shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
      <table className="w-full">
        <thead>
          <tr className="border-b border-slate-100">
            <th className="text-left px-6 py-3.5 text-xs font-semibold
              uppercase tracking-wider text-slate-400 font-[DM_Sans]">
              Indicateur
            </th>
            <th className="text-right px-6 py-3.5 text-xs font-semibold
              uppercase tracking-wider text-slate-400 font-[DM_Sans]">
              Valeur
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i} className={`
              border-b border-slate-50 last:border-0
              transition-colors duration-100
              hover:bg-[rgba(12,65,154,0.04)]
              ${row.highlight ? 'bg-primary-50/50' : ''}
            `}>
              <td className="px-6 py-3.5 flex items-center gap-3">
                {row.icon && <span className="text-slate-400">{row.icon}</span>}
                <span className={`text-sm font-[Source_Sans_3]
                  ${row.highlight ? 'font-semibold text-slate-800' : 'text-slate-600'}`}>
                  {row.label}
                </span>
              </td>
              <td className={`px-6 py-3.5 text-right text-base font-semibold
                font-[DM_Sans] tabular-nums
                ${row.positive ? 'text-emerald-600' : ''}
                ${row.negative ? 'text-red-600' : ''}
                ${!row.positive && !row.negative ? 'text-slate-800' : ''}
              `}>
                {typeof row.value === 'number'
                  ? row.value.toLocaleString('fr-FR')
                  : row.value}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
