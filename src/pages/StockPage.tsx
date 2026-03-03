import { useEffect, useMemo, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X } from 'lucide-react';
import { useInvoke } from '../hooks/useInvoke';
import { useECharts } from '../hooks/useECharts';
import { useFilterStore } from '../stores/filterStore';
import { KpiCards } from '../components/stock/KpiCards';
import { TechnicianTable } from '../components/stock/TechnicianTable';
import { Card } from '../components/shared/Card';
import type { StockOverview, TechnicianStats, CategoryTree, CategoryNode, TicketSummary, AgeRangeCount } from '../types/kpi';
import type { EChartsCoreOption } from 'echarts/core';
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

/* ── Distribution de l'âge des tickets ── */
const AGE_COLORS = ['#2E7D32', '#1565C0', '#FF8F00', '#6A1B9A', '#C62828'];

function AgeDistributionChart({ data }: { data: AgeRangeCount[] }) {
  const option = useMemo<EChartsCoreOption>(() => {
    const labels = data.map((d) => d.label);
    const counts = data.map((d) => d.count);

    return {
      tooltip: {
        trigger: 'axis' as const,
        axisPointer: { type: 'shadow' as const },
        formatter: (params: unknown) => {
          const p = (params as Array<{ name: string; value: number; dataIndex: number }>)[0];
          if (!p) return '';
          const tranche = data[p.dataIndex];
          return `<div style="font-size:13px;color:#1E293B">
            <strong>${p.name}</strong><br/>
            ${p.value.toLocaleString('fr-FR')} ticket(s)
            ${tranche ? ` (${tranche.percentage}%)` : ''}
          </div>`;
        },
      },
      grid: { left: 100, right: 60, top: 10, bottom: 20 },
      xAxis: { type: 'value' as const },
      yAxis: {
        type: 'category' as const,
        data: labels,
        inverse: true,
        axisLabel: { fontSize: 12, fontFamily: 'DM Sans' },
      },
      series: [
        {
          type: 'bar' as const,
          data: counts.map((val, idx) => ({
            value: val,
            itemStyle: { color: AGE_COLORS[idx % AGE_COLORS.length], borderRadius: [0, 6, 6, 0] },
          })),
          barWidth: '55%',
          label: {
            show: true,
            position: 'right' as const,
            fontSize: 11,
            color: '#64748B',
            formatter: (params: { value: number; dataIndex: number }) => {
              const t = data[params.dataIndex];
              return `${params.value.toLocaleString('fr-FR')} (${t.percentage}%)`;
            },
          },
        },
      ],
    };
  }, [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');

  return <div ref={chartRef} className="h-[220px] w-full" />;
}

/* ── Drawer tickets non assignés ── */
function UnassignedDrawer({ onClose }: { onClose: () => void }) {
  const [tickets, setTickets] = useState<TicketSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<TicketSummary[]>('get_unassigned_tickets')
      .then(setTickets)
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/20 backdrop-blur-[2px]" onClick={onClose} />

      {/* Panel */}
      <div className="relative w-[700px] max-w-[90vw] bg-white shadow-[0_20px_40px_rgba(0,0,0,0.14),0_8px_20px_rgba(0,0,0,0.10)] flex flex-col animate-slide-in-right">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-100">
          <div>
            <h2 className="text-lg font-bold font-[DM_Sans] text-slate-800">
              Tickets non assignes
            </h2>
            {!loading && (
              <p className="text-sm text-slate-400 mt-0.5">
                {tickets.length.toLocaleString('fr-FR')} ticket(s)
              </p>
            )}
          </div>
          <button
            onClick={onClose}
            className="w-8 h-8 rounded-lg flex items-center justify-center hover:bg-slate-100 transition-colors"
          >
            <X size={18} className="text-slate-400" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          {loading ? (
            <div className="flex items-center justify-center py-24 text-sm text-slate-400">
              Chargement...
            </div>
          ) : tickets.length === 0 ? (
            <div className="flex items-center justify-center py-24 text-sm text-slate-400">
              Aucun ticket non assigne
            </div>
          ) : (
            <table className="w-full text-sm">
              <thead className="sticky top-0 bg-slate-50/95 backdrop-blur-sm">
                <tr>
                  <th className="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-400">ID</th>
                  <th className="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-400">Titre</th>
                  <th className="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-400">Statut</th>
                  <th className="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-400">Type</th>
                  <th className="text-left px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-400">Groupe</th>
                  <th className="text-right px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-400">Age (j)</th>
                </tr>
              </thead>
              <tbody>
                {tickets.map((t) => {
                  const age = t.ancienneteJours;
                  const ageCls = age != null && age > 90
                    ? 'text-red-600 font-medium'
                    : age != null && age > 30
                      ? 'text-amber-600'
                      : 'text-slate-800';
                  return (
                    <tr
                      key={t.id}
                      className="border-b border-slate-50 last:border-0 hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100"
                    >
                      <td className="px-4 py-2.5 font-[DM_Sans] font-semibold tabular-nums text-slate-600">{t.id}</td>
                      <td className="px-4 py-2.5 text-slate-800 max-w-[260px] truncate">{t.titre}</td>
                      <td className="px-4 py-2.5 text-slate-600">{t.statut}</td>
                      <td className="px-4 py-2.5 text-slate-600">{t.typeTicket}</td>
                      <td className="px-4 py-2.5 text-slate-600 max-w-[140px] truncate">{t.groupePrincipal ?? '—'}</td>
                      <td className={`px-4 py-2.5 text-right font-[DM_Sans] font-semibold tabular-nums ${ageCls}`}>
                        {age ?? '—'}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}

function StockPage() {
  const { statut, typeTicket, groupe, resetFilters, setStatut, setTypeTicket, setGroupe } = useFilterStore();
  const [showUnassigned, setShowUnassigned] = useState(false);

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
          {overviewHook.data && (
            <KpiCards
              overview={overviewHook.data}
              onUnassignedClick={() => setShowUnassigned(true)}
            />
          )}
        </div>

        {/* Age distribution */}
        {overviewHook.data && overviewHook.data.parAnciennete.length > 0 && (
          <div className="animate-fade-slide-up animation-delay-150">
            <Card>
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700">
                  Distribution de l'age des tickets
                </h3>
                <span className="text-xs text-slate-400">
                  {overviewHook.data.totalVivants.toLocaleString('fr-FR')} tickets vivants
                </span>
              </div>
              <AgeDistributionChart data={overviewHook.data.parAnciennete} />
            </Card>
          </div>
        )}

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

      {/* Drawer tickets non assignés */}
      {showUnassigned && <UnassignedDrawer onClose={() => setShowUnassigned(false)} />}
    </div>
  );
}

export default StockPage;
