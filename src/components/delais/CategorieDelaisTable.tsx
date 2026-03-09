import { useState, useMemo } from 'react';
import { ArrowUpDown } from 'lucide-react';
import type { CategorieDelais } from '../../types/delais';

type SortKey = keyof CategorieDelais;
type SortDir = 'asc' | 'desc';

interface Props {
  data: CategorieDelais[];
  loading: boolean;
}

export function CategorieDelaisTable({ data, loading }: Props) {
  const [sortKey, setSortKey] = useState<SortKey>('totalResolus');
  const [sortDir, setSortDir] = useState<SortDir>('desc');

  const sorted = useMemo(() => {
    return [...data].sort((a, b) => {
      const va = a[sortKey];
      const vb = b[sortKey];
      if (typeof va === 'number' && typeof vb === 'number') {
        return sortDir === 'asc' ? va - vb : vb - va;
      }
      return sortDir === 'asc'
        ? String(va).localeCompare(String(vb), 'fr')
        : String(vb).localeCompare(String(va), 'fr');
    });
  }, [data, sortKey, sortDir]);

  const toggle = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir(key === 'categorie' ? 'asc' : 'desc');
    }
  };

  const columns: { key: SortKey; label: string; align: string; format: (v: CategorieDelais) => string }[] = [
    { key: 'categorie', label: 'Categorie', align: 'text-left', format: (r) => r.categorie },
    { key: 'totalResolus', label: 'Tickets resolus', align: 'text-right', format: (r) => r.totalResolus.toLocaleString('fr-FR') },
    { key: 'mttrJours', label: 'MTTR (j)', align: 'text-right', format: (r) => r.mttrJours.toFixed(1) },
    { key: 'medianeJours', label: 'Mediane (j)', align: 'text-right', format: (r) => r.medianeJours.toFixed(1) },
    { key: 'taux24h', label: 'Taux 24h', align: 'text-right', format: (r) => `${r.taux24h}%` },
    { key: 'taux48h', label: 'Taux 48h', align: 'text-right', format: (r) => `${r.taux48h}%` },
  ];

  if (loading) {
    return <p className="text-sm text-slate-400 py-8 text-center">Chargement...</p>;
  }

  if (data.length === 0) {
    return <p className="text-sm text-slate-400 py-8 text-center">Aucune donnee pour cette selection</p>;
  }

  return (
    <div className="bg-white rounded-2xl overflow-hidden shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
      <table className="w-full">
        <thead>
          <tr>
            {columns.map((col) => (
              <th
                key={col.key}
                onClick={() => toggle(col.key)}
                className={`px-4 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans] cursor-pointer select-none hover:text-slate-600 transition-colors ${col.align}`}
              >
                <span className="inline-flex items-center gap-1">
                  {col.label}
                  <ArrowUpDown size={12} className={sortKey === col.key ? 'text-[#0C419A]' : 'text-slate-300'} />
                </span>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {sorted.map((row) => (
            <tr key={row.categorie} className="hover:bg-[#0C419A]/[0.04] transition-colors">
              <td className="px-4 py-3 text-sm font-[Source_Sans_3] text-slate-700 text-left">{row.categorie}</td>
              <td className="px-4 py-3 text-sm font-[DM_Sans] font-semibold tabular-nums text-right">{row.totalResolus.toLocaleString('fr-FR')}</td>
              <td className="px-4 py-3 text-sm font-[DM_Sans] font-semibold tabular-nums text-right">{row.mttrJours.toFixed(1)}</td>
              <td className="px-4 py-3 text-sm font-[DM_Sans] font-semibold tabular-nums text-right">{row.medianeJours.toFixed(1)}</td>
              <td className="px-4 py-3 text-sm font-[DM_Sans] font-semibold tabular-nums text-right">{row.taux24h}%</td>
              <td className="px-4 py-3 text-sm font-[DM_Sans] font-semibold tabular-nums text-right">{row.taux48h}%</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
