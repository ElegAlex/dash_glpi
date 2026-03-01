import { useEffect, useMemo } from 'react';
import { useInvoke } from '../hooks/useInvoke';
import { useFilterStore } from '../stores/filterStore';
import { KpiCards } from '../components/stock/KpiCards';
import { TechnicianTable } from '../components/stock/TechnicianTable';
import { Card } from '../components/shared/Card';
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

  const statutOptions = ['Nouveau', 'En cours (Attribue)', 'En cours (Planifie)', 'En attente'];
  const typeOptions = ['Incident', 'Demande'];

  const filteredTechs = useMemo(() => {
    let rows = techHook.data ?? [];
    if (statut) rows = rows.filter((t) => t.enCours > 0 || t.total > 0);
    if (typeTicket === 'Incident') rows = rows.filter((t) => t.incidents > 0);
    if (typeTicket === 'Demande') rows = rows.filter((t) => t.demandes > 0);
    return rows;
  }, [techHook.data, statut, typeTicket]);

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
              Dashboard Stock
            </h1>
            <p className="text-sm text-slate-400 mt-1">
              Vue d'ensemble du stock de tickets vivants
            </p>
          </div>
          {overviewHook.loading && (
            <span className="text-sm text-slate-400">Chargement...</span>
          )}
        </div>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        {overviewHook.error && (
          <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
            {overviewHook.error}
          </div>
        )}

        <div className="animate-fade-slide-up">
          {overviewHook.data && <KpiCards overview={overviewHook.data} />}
        </div>

        {/* Filter bar */}
        <div className="animate-fade-slide-up animation-delay-150">
          <Card padding="sm">
            <div className="flex flex-wrap items-center gap-3">
              <span className="text-sm font-medium text-slate-500">Filtres :</span>

              <select
                value={statut ?? ''}
                onChange={(e) => setStatut(e.target.value || null)}
                className="rounded-lg bg-slate-50 px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
              >
                <option value="">Tous statuts</option>
                {statutOptions.map((s) => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>

              <select
                value={typeTicket ?? ''}
                onChange={(e) => setTypeTicket(e.target.value || null)}
                className="rounded-lg bg-slate-50 px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
              >
                <option value="">Tous types</option>
                {typeOptions.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>

              <select
                value={groupe ?? ''}
                onChange={(e) => setGroupe(e.target.value || null)}
                className="rounded-lg bg-slate-50 px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
              >
                <option value="">Tous groupes</option>
                {groupOptions.map((g) => (
                  <option key={g} value={g}>{g}</option>
                ))}
              </select>

              {(statut || typeTicket || groupe) && (
                <button
                  onClick={resetFilters}
                  className="rounded-lg bg-slate-100 px-3 py-1.5 text-sm text-slate-500 hover:bg-slate-200 transition-colors"
                >
                  Reinitialiser
                </button>
              )}
            </div>
          </Card>
        </div>

        {/* Technician table */}
        <div className="animate-fade-slide-up animation-delay-300 space-y-2">
          <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700">Charge par technicien</h2>
          {techHook.loading ? (
            <div className="py-8 text-center text-sm text-slate-400">Chargement...</div>
          ) : (
            <TechnicianTable data={filteredTechs} />
          )}
        </div>
      </div>
    </div>
  );
}

export default StockPage;
