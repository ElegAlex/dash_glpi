import { useEffect, useMemo, useState, useCallback } from 'react';
import { useInvoke } from '../hooks/useInvoke';
import { useECharts } from '../hooks/useECharts';
import { useFilterStore } from '../stores/filterStore';
import { KpiCards } from '../components/stock/KpiCards';
import { TechnicianTable } from '../components/stock/TechnicianTable';
import { Card } from '../components/shared/Card';
import type { StockOverview, TechnicianStats, CategoryTree, CategoryNode } from '../types/kpi';
import '../lib/echarts-theme';

const PALETTE = ['#1565C0', '#2E7D32', '#FF8F00', '#6A1B9A', '#00838F', '#C62828', '#4E342E', '#37474F'];

function CategoryBarChart({ tree }: { tree: CategoryTree }) {
  const [drillPath, setDrillPath] = useState<CategoryNode[]>([]);

  const currentNodes = useMemo(() => {
    if (drillPath.length === 0) return tree.nodes;
    return drillPath[drillPath.length - 1].children;
  }, [tree, drillPath]);

  const sorted = useMemo(
    () => [...currentNodes].sort((a, b) => b.count - a.count).slice(0, 15),
    [currentNodes],
  );

  const option = useMemo(() => {
    const labels = sorted.map((n) => n.name);
    const values = sorted.map((n) => n.count);

    return {
      tooltip: {
        trigger: 'axis' as const,
        axisPointer: { type: 'shadow' as const },
      },
      grid: { left: 200, right: 40, top: 10, bottom: 20 },
      xAxis: { type: 'value' as const },
      yAxis: {
        type: 'category' as const,
        data: labels,
        inverse: true,
        axisLabel: { fontSize: 11, width: 180, overflow: 'truncate' as const },
      },
      series: [
        {
          type: 'bar' as const,
          data: values.map((val, idx) => ({
            value: val,
            itemStyle: { color: PALETTE[idx % PALETTE.length], borderRadius: [0, 6, 6, 0] },
          })),
          barWidth: '60%',
          cursor: 'pointer',
          label: {
            show: true,
            position: 'right' as const,
            fontSize: 11,
            color: '#64748B',
            formatter: (params: { value: number }) => params.value.toLocaleString('fr-FR'),
          },
        },
      ],
    };
  }, [sorted]);

  const onEvents = useMemo(() => ({
    click: (params: unknown) => {
      const p = params as { dataIndex?: number };
      if (p.dataIndex == null) return;
      const clicked = sorted[p.dataIndex];
      if (clicked && clicked.children.length > 0) {
        setDrillPath((prev) => [...prev, clicked]);
      }
    },
  }), [sorted]);

  const { chartRef } = useECharts(option, onEvents, 'cpam-material');

  const handleBreadcrumb = useCallback((depth: number) => {
    setDrillPath((prev) => prev.slice(0, depth));
  }, []);

  const barCount = Math.min(sorted.length, 15);

  return (
    <div>
      {drillPath.length > 0 && (
        <div className="flex items-center gap-1 mb-3 text-xs">
          <button
            onClick={() => handleBreadcrumb(0)}
            className="text-primary-500 hover:underline cursor-pointer font-medium"
          >
            Toutes
          </button>
          {drillPath.map((node, i) => (
            <span key={node.fullPath} className="flex items-center gap-1">
              <span className="text-slate-300">/</span>
              {i < drillPath.length - 1 ? (
                <button
                  onClick={() => handleBreadcrumb(i + 1)}
                  className="text-primary-500 hover:underline cursor-pointer font-medium"
                >
                  {node.name}
                </button>
              ) : (
                <span className="text-slate-600 font-semibold">{node.name}</span>
              )}
            </span>
          ))}
        </div>
      )}
      <div ref={chartRef} style={{ height: Math.max(220, barCount * 32), width: '100%' }} />
    </div>
  );
}

function StockPage() {
  const { statut, typeTicket, groupe, resetFilters, setStatut, setTypeTicket, setGroupe } = useFilterStore();

  const overviewHook = useInvoke<StockOverview>();
  const techHook = useInvoke<TechnicianStats[]>();
  const catHook = useInvoke<CategoryTree>();

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

  useEffect(() => {
    catHook.execute('get_categories_tree', { request: { scope: 'all' } });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

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

        {/* Category distribution */}
        {catHook.data && catHook.data.nodes.length > 0 && (
          <div className="animate-fade-slide-up animation-delay-300">
            <Card>
              <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 mb-3">
                {catHook.data.source === 'categorie' ? 'Repartition par categorie' : 'Repartition par groupe'}
              </h3>
              <CategoryBarChart tree={catHook.data} />
            </Card>
          </div>
        )}

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
