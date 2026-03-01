import { useEffect, useMemo } from 'react';
import { useInvoke } from '../hooks/useInvoke';
import { useFilterStore } from '../stores/filterStore';
import { KpiCards } from '../components/stock/KpiCards';
import { TechnicianTable } from '../components/stock/TechnicianTable';
import type { StockOverview, TechnicianStats } from '../types/kpi';

function StockPage() {
  const { statut, typeTicket, groupe, resetFilters, setStatut, setTypeTicket, setGroupe } = useFilterStore();

  const overviewHook = useInvoke<StockOverview>();
  const techHook = useInvoke<TechnicianStats[]>();

  const filters = useMemo(
    () => ({ statut: statut || null, typeTicket: typeTicket || null, groupe: groupe || null }),
    [statut, typeTicket, groupe],
  );

  useEffect(() => {
    overviewHook.execute('get_stock_overview', { filters });
  }, [filters]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    techHook.execute('get_stock_by_technician', { filters });
  }, [filters]); // eslint-disable-line react-hooks/exhaustive-deps

  // Extract unique filter values from technician data
  const groupOptions = useMemo(() => {
    const raw = techHook.data ?? [];
    const seen = new Set<string>();
    raw.forEach((t) => {
      if (t.technicien) seen.add(t.technicien.split(' > ')[0]);
    });
    return Array.from(seen).sort();
  }, [techHook.data]);

  const statutOptions = ['Nouveau', 'En cours (Attribué)', 'En cours (Planifié)', 'En attente'];
  const typeOptions = ['Incident', 'Demande'];

  const filteredTechs = useMemo(() => {
    let rows = techHook.data ?? [];
    if (statut) rows = rows.filter((t) => t.enCours > 0 || t.total > 0);
    if (typeTicket === 'Incident') rows = rows.filter((t) => t.incidents > 0);
    if (typeTicket === 'Demande') rows = rows.filter((t) => t.demandes > 0);
    return rows;
  }, [techHook.data, statut, typeTicket]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-[#1a1f2e]">Dashboard Stock</h1>
        {overviewHook.loading && (
          <span className="text-sm text-[#6e7891]">Chargement…</span>
        )}
      </div>

      {overviewHook.error && (
        <div className="rounded-md bg-[#fef2f2] border border-[#ce0500] px-4 py-3 text-sm text-[#af0400]">
          {overviewHook.error}
        </div>
      )}

      {overviewHook.data && <KpiCards overview={overviewHook.data} />}

      {/* Filter bar */}
      <div className="flex flex-wrap items-center gap-3 rounded-lg border border-[#e2e6ee] bg-white px-4 py-3 shadow-[0_1px_2px_0_rgb(26_31_46/0.04)]">
        <span className="text-sm font-medium text-[#525d73]">Filtres :</span>

        <select
          value={statut ?? ''}
          onChange={(e) => setStatut(e.target.value || null)}
          className="rounded-md border border-[#cdd3df] bg-white px-3 py-1.5 text-sm text-[#1a1f2e] focus:border-[#0C419A] focus:outline-none"
        >
          <option value="">Tous statuts</option>
          {statutOptions.map((s) => (
            <option key={s} value={s}>{s}</option>
          ))}
        </select>

        <select
          value={typeTicket ?? ''}
          onChange={(e) => setTypeTicket(e.target.value || null)}
          className="rounded-md border border-[#cdd3df] bg-white px-3 py-1.5 text-sm text-[#1a1f2e] focus:border-[#0C419A] focus:outline-none"
        >
          <option value="">Tous types</option>
          {typeOptions.map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>

        <select
          value={groupe ?? ''}
          onChange={(e) => setGroupe(e.target.value || null)}
          className="rounded-md border border-[#cdd3df] bg-white px-3 py-1.5 text-sm text-[#1a1f2e] focus:border-[#0C419A] focus:outline-none"
        >
          <option value="">Tous groupes</option>
          {groupOptions.map((g) => (
            <option key={g} value={g}>{g}</option>
          ))}
        </select>

        {(statut || typeTicket || groupe) && (
          <button
            onClick={resetFilters}
            className="rounded-md bg-[#f1f3f7] px-3 py-1.5 text-sm text-[#525d73] hover:bg-[#e2e6ee] transition-colors"
          >
            Réinitialiser
          </button>
        )}
      </div>

      {/* Technician table */}
      <div className="space-y-2">
        <h2 className="text-base font-medium text-[#1a1f2e]">Charge par technicien</h2>
        {techHook.loading ? (
          <div className="py-8 text-center text-sm text-[#6e7891]">Chargement…</div>
        ) : (
          <TechnicianTable data={filteredTechs} />
        )}
      </div>
    </div>
  );
}

export default StockPage;
