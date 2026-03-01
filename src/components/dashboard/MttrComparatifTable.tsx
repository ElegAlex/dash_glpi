import { useMemo } from 'react';
import type { MttrParDimension } from '../../types/dashboard';

interface MttrComparatifTableProps {
  data: MttrParDimension[];
}

export function MttrComparatifTable({ data }: MttrComparatifTableProps) {
  const sorted = useMemo(
    () => [...data].sort((a, b) => b.count - a.count).slice(0, 20),
    [data],
  );

  if (sorted.length === 0) {
    return (
      <div className="py-8 text-center text-sm text-slate-400">
        Aucune donnee technicien disponible.
      </div>
    );
  }

  return (
    <div className="overflow-auto rounded-2xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
      <table className="w-full">
        <thead>
          <tr className="bg-slate-50">
            <th className="text-left px-5 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400">
              Technicien
            </th>
            <th className="text-right px-5 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400">
              Nb resolus
            </th>
            <th className="text-right px-5 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400">
              MTTR moyen (j)
            </th>
            <th className="text-right px-5 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400">
              Mediane (j)
            </th>
            <th className="text-right px-5 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400">
              % total
            </th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((row) => (
            <tr
              key={row.label}
              className="hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100"
            >
              <td className="px-5 py-3 text-sm font-[Source_Sans_3] text-slate-800">
                {row.label}
              </td>
              <td className="px-5 py-3 text-right font-[DM_Sans] font-semibold tabular-nums text-sm text-slate-800">
                {row.count.toLocaleString('fr-FR')}
              </td>
              <td className="px-5 py-3 text-right font-[DM_Sans] font-semibold tabular-nums text-sm text-slate-800">
                {row.mttrJours.toFixed(1)}
              </td>
              <td className="px-5 py-3 text-right font-[DM_Sans] font-semibold tabular-nums text-sm text-slate-800">
                {row.medianeJours.toFixed(1)}
              </td>
              <td className="px-5 py-3 text-right font-[DM_Sans] font-semibold tabular-nums text-sm text-slate-400">
                {row.pourcentageTotal.toFixed(1)}%
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
