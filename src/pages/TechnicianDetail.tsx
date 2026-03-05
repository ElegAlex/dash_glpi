import { useEffect, useRef, useMemo, useState } from 'react';
import { useParams, useNavigate } from 'react-router';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  createColumnHelper,
  type SortingState,
} from '@tanstack/react-table';
import { useVirtualizer } from '@tanstack/react-virtual';
import { save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { useInvoke } from '../hooks/useInvoke';
import { useECharts } from '../hooks/useECharts';
import type { GLPITicket } from '../types/tickets';
import type { ExportResult, TechHistory, TechHistoryPeriod } from '../types/config';
import { KpiCard } from '../components/shared/KpiCard';
import { ArrowDownToLine, ArrowUpFromLine, Clock, Package } from 'lucide-react';
import type { EChartsCoreOption } from 'echarts/core';

type Granularity = 'day' | 'week' | 'month' | 'quarter' | 'year';
const GRAN_OPTIONS: { value: Granularity; label: string }[] = [
  { value: 'day', label: 'Jour' },
  { value: 'week', label: 'Semaine' },
  { value: 'month', label: 'Mois' },
  { value: 'quarter', label: 'Trimestre' },
  { value: 'year', label: 'Annee' },
];

const columnHelper = createColumnHelper<GLPITicket>();

const COLUMNS = [
  columnHelper.accessor('id', { header: 'ID', size: 80 }),
  columnHelper.accessor('titre', { header: 'Titre', size: 280 }),
  columnHelper.accessor('statut', { header: 'Statut', size: 150 }),
  columnHelper.accessor('typeTicket', { header: 'Type', size: 100 }),
  columnHelper.accessor('priorite', {
    header: 'Priorite',
    size: 80,
    cell: (info) => info.getValue() ?? '—',
  }),
  columnHelper.accessor('groupePrincipal', {
    header: 'Groupe',
    size: 200,
    cell: (info) => info.getValue() ?? '—',
  }),
  columnHelper.accessor('dateOuverture', {
    header: 'Ouverture',
    size: 110,
    cell: (info) => {
      const v = info.getValue();
      return v ? new Date(v).toLocaleDateString('fr-FR') : '—';
    },
  }),
  columnHelper.accessor('ancienneteJours', {
    header: 'Age (j)',
    size: 80,
    cell: (info) => {
      const v = info.getValue();
      if (v === null || v === undefined) return '—';
      const cls = v > 90 ? 'text-danger-500 font-medium' : v > 30 ? 'text-warning-500' : 'text-slate-800';
      return <span className={cls}>{v}</span>;
    },
  }),
];

/* ── Charts sub-component (mounts AFTER data is loaded → useECharts inits correctly) ── */
function HistoriqueCharts({ periodes }: { periodes: TechHistoryPeriod[] }) {
  const volumeOption = useMemo<EChartsCoreOption>(() => {
    const keys = periodes.map((p) => p.periodKey);
    return {
      tooltip: { trigger: 'axis', axisPointer: { type: 'cross' } },
      legend: { data: ['Entrants', 'Sortants', 'Stock cumule'], bottom: 0 },
      grid: { left: 55, right: 55, top: 30, bottom: 50 },
      xAxis: { type: 'category', data: keys, axisLabel: { fontSize: 11 } },
      yAxis: [
        { type: 'value', name: 'Volume', position: 'left' },
        { type: 'value', name: 'Stock', position: 'right' },
      ],
      series: [
        {
          name: 'Entrants',
          type: 'bar',
          yAxisIndex: 0,
          data: periodes.map((p) => p.entrants),
          itemStyle: { color: '#1565C0', borderRadius: [6, 6, 0, 0] },
        },
        {
          name: 'Sortants',
          type: 'bar',
          yAxisIndex: 0,
          data: periodes.map((p) => p.sortants),
          itemStyle: { color: '#2E7D32', borderRadius: [6, 6, 0, 0] },
        },
        {
          name: 'Stock cumule',
          type: 'line',
          yAxisIndex: 1,
          data: periodes.map((p) => p.stockCumule),
          smooth: true,
          symbol: 'circle',
          symbolSize: 6,
          lineStyle: { width: 2.5, color: '#FF8F00' },
          itemStyle: { color: '#FF8F00', borderColor: '#fff', borderWidth: 2 },
          areaStyle: {
            color: {
              type: 'linear', x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: 'rgba(255,143,0,0.15)' },
                { offset: 1, color: 'rgba(255,143,0,0.02)' },
              ],
            },
          },
        },
      ],
    };
  }, [periodes]);

  const withMttr = useMemo(() => periodes.filter((p) => p.mttrJours != null), [periodes]);

  const mttrOption = useMemo<EChartsCoreOption>(() => {
    if (withMttr.length === 0) return {};
    return {
      tooltip: {
        trigger: 'axis',
        formatter: (params: Array<{ name: string; value: number; marker: string }>) => {
          const p = params[0];
          return p ? `${p.name}<br/>${p.marker} MTTR: <b>${p.value.toFixed(1)} j</b>` : '';
        },
      },
      grid: { left: 55, right: 20, top: 30, bottom: 30 },
      xAxis: { type: 'category', data: withMttr.map((p) => p.periodKey), axisLabel: { fontSize: 11 } },
      yAxis: { type: 'value', name: 'Jours' },
      series: [
        {
          name: 'MTTR',
          type: 'line',
          data: withMttr.map((p) => Math.round((p.mttrJours ?? 0) * 10) / 10),
          smooth: true,
          symbol: 'circle',
          symbolSize: 6,
          lineStyle: { width: 2.5, color: '#6A1B9A' },
          itemStyle: { color: '#6A1B9A', borderColor: '#fff', borderWidth: 2 },
          areaStyle: {
            color: {
              type: 'linear', x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: 'rgba(106,27,154,0.15)' },
                { offset: 1, color: 'rgba(106,27,154,0.02)' },
              ],
            },
          },
        },
      ],
    };
  }, [withMttr]);

  const { chartRef: volumeRef } = useECharts(volumeOption, undefined, 'cpam-material');
  const { chartRef: mttrRef } = useECharts(withMttr.length > 0 ? mttrOption : {}, undefined, 'cpam-material');

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-5 animate-fade-slide-up animation-delay-150">
      <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-5">
        <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 mb-3">Volumetrie</h3>
        <div ref={volumeRef} style={{ height: 300 }} />
      </div>
      <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-5">
        <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 mb-3">MTTR — Delai moyen de resolution</h3>
        {withMttr.length > 0 ? (
          <div ref={mttrRef} style={{ height: 300 }} />
        ) : (
          <div className="h-[300px] flex items-center justify-center text-sm text-slate-400">
            Aucun ticket resolu sur la periode
          </div>
        )}
      </div>
    </div>
  );
}

function HistoriqueTab({ technicien }: { technicien: string }) {
  const { data, loading, error, execute } = useInvoke<TechHistory>();
  const [granularity, setGranularity] = useState<Granularity>('month');

  useEffect(() => {
    execute('get_technician_history', { technicien, granularity });
  }, [technicien, granularity]); // eslint-disable-line react-hooks/exhaustive-deps

  if (loading && !data) {
    return (
      <div className="py-12 text-center text-sm text-slate-400">Chargement de l'historique...</div>
    );
  }

  if (error) {
    return (
      <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
        {error}
      </div>
    );
  }

  if (!data || data.periodes.length === 0) {
    return (
      <div className="py-12 text-center text-sm text-slate-400">
        Aucune donnee historique disponible
      </div>
    );
  }

  const { kpi, periodes } = data;

  return (
    <div className="space-y-5 pt-4">
      {/* Granularity selector */}
      <div className="flex items-center justify-between animate-fade-slide-up">
        <div />
        <div className="flex items-center gap-1 bg-white rounded-xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-1">
          {GRAN_OPTIONS.map((g) => (
            <button
              key={g.value}
              onClick={() => setGranularity(g.value)}
              className={`px-3 py-1.5 rounded-lg text-xs font-semibold font-[DM_Sans] transition-colors duration-150 ${
                granularity === g.value
                  ? 'bg-primary-500 text-white shadow-sm'
                  : 'text-slate-400 hover:text-slate-600'
              }`}
            >
              {g.label}
            </button>
          ))}
        </div>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-5 animate-fade-slide-up">
        <KpiCard
          label="Entrants"
          value={kpi.totalEntrants}
          accentColor="#1565C0"
          icon={<ArrowDownToLine size={18} color="#1565C0" />}
        />
        <KpiCard
          label="Sortants"
          value={kpi.totalSortants}
          accentColor="#2E7D32"
          icon={<ArrowUpFromLine size={18} color="#2E7D32" />}
        />
        <KpiCard
          label="MTTR moyen"
          value={kpi.mttrJours != null ? Math.round(kpi.mttrJours) : '—'}
          format="days"
          accentColor="#FF8F00"
          icon={<Clock size={18} color="#FF8F00" />}
        />
        <KpiCard
          label="Stock actuel"
          value={kpi.stockActuel}
          accentColor={kpi.stockActuel > 20 ? '#C62828' : '#64748B'}
          icon={<Package size={18} color={kpi.stockActuel > 20 ? '#C62828' : '#64748B'} />}
        />
      </div>

      {/* Charts */}
      <HistoriqueCharts periodes={periodes} />

      {/* Recap table */}
      <div className="animate-fade-slide-up animation-delay-300 rounded-2xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] overflow-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-100">
              <th className="px-6 py-3.5 text-left text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">Periode</th>
              <th className="px-4 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">Entrants</th>
              <th className="px-4 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">Sortants</th>
              <th className="px-4 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">Delta</th>
              <th className="px-4 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">Stock</th>
              <th className="px-6 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">MTTR (j)</th>
            </tr>
          </thead>
          <tbody>
            {periodes.map((p: TechHistoryPeriod) => {
              const delta = p.entrants - p.sortants;
              return (
                <tr key={p.periodKey} className="border-b border-slate-50 last:border-0 hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100">
                  <td className="px-6 py-3 text-slate-800 font-medium">{p.periodKey}</td>
                  <td className="px-4 py-3 text-right text-slate-800 font-semibold font-[DM_Sans] tabular-nums">{p.entrants}</td>
                  <td className="px-4 py-3 text-right text-slate-800 font-semibold font-[DM_Sans] tabular-nums">{p.sortants}</td>
                  <td className={`px-4 py-3 text-right font-semibold font-[DM_Sans] tabular-nums ${delta > 0 ? 'text-red-600' : delta < 0 ? 'text-emerald-600' : 'text-slate-400'}`}>
                    {delta > 0 ? `+${delta}` : delta}
                  </td>
                  <td className="px-4 py-3 text-right text-slate-800 font-semibold font-[DM_Sans] tabular-nums">{p.stockCumule}</td>
                  <td className="px-6 py-3 text-right text-slate-500 font-[DM_Sans] tabular-nums">
                    {p.mttrJours != null ? p.mttrJours.toFixed(1) : '—'}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function TechnicianDetail() {
  const { technicien } = useParams<{ technicien: string }>();
  const navigate = useNavigate();
  const { data, loading, error, execute } = useInvoke<GLPITicket[]>();
  const [sorting, setSorting] = useState<SortingState>([{ id: 'ancienneteJours', desc: true }]);
  const [exporting, setExporting] = useState(false);
  const [activeTab, setActiveTab] = useState<'tickets' | 'historique'>('tickets');

  const decodedTech = useMemo(() => {
    return technicien ? decodeURIComponent(technicien) : '';
  }, [technicien]);

  useEffect(() => {
    if (decodedTech) {
      execute('get_technician_tickets', { technician: decodedTech });
    }
  }, [decodedTech]); // eslint-disable-line react-hooks/exhaustive-deps

  const tickets = useMemo(() => data ?? [], [data]);

  const table = useReactTable({
    data: tickets,
    columns: COLUMNS,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  const { rows } = table.getRowModel();
  const tableContainerRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => 35,
    overscan: 10,
    useFlushSync: false,
  });

  const over90Count = useMemo(
    () => tickets.filter((t) => (t.ancienneteJours ?? 0) > 90).length,
    [tickets],
  );

  const handleExportPlan = async () => {
    if (!decodedTech) return;
    const safeName = decodedTech.replace(/\s+/g, '_');
    const path = await save({
      defaultPath: `plan_action_${safeName}.xlsx`,
      filters: [{ name: 'Excel', extensions: ['xlsx'] }],
    });
    if (!path) return;
    setExporting(true);
    try {
      await invoke<ExportResult>('export_excel_plan_action', { technicien: decodedTech, path });
    } finally {
      setExporting(false);
    }
  };

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        {/* Breadcrumb */}
        <nav className="flex items-center gap-1 text-sm text-slate-500 mb-2">
          <button
            onClick={() => navigate('/stock')}
            className="hover:text-primary-500 font-medium transition-colors"
          >
            Stock
          </button>
          <span className="text-slate-400">/</span>
          <span className="font-medium text-slate-800">{decodedTech || '...'}</span>
        </nav>

        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
              {decodedTech}
            </h1>
            {!loading && tickets.length > 0 && (
              <p className="mt-1 text-sm text-slate-400">
                {tickets.length.toLocaleString('fr-FR')} ticket(s) vivant(s)
                {over90Count > 0 && (
                  <span className="ml-2 inline-flex items-center rounded-lg bg-danger-50 px-2 py-0.5 text-xs font-medium text-danger-500">
                    {over90Count} &gt; 90 j
                  </span>
                )}
              </p>
            )}
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleExportPlan}
              disabled={exporting || loading || tickets.length === 0}
              className="rounded-xl bg-primary-500 px-3 py-1.5 text-sm font-medium text-white hover:bg-primary-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
            >
              {exporting ? 'Export...' : "Exporter le plan d'action"}
            </button>
            <button
              onClick={() => navigate('/stock')}
              className="rounded-xl bg-white px-3 py-1.5 text-sm text-slate-600 hover:bg-slate-50 transition-colors shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
            >
              Retour
            </button>
          </div>
        </div>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        {error && (
          <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
            {error}
          </div>
        )}

        {/* Tab bar */}
        <div className="animate-fade-slide-up flex bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] overflow-hidden">
          <button
            className={`px-5 py-3 text-sm font-medium transition-colors ${activeTab === 'tickets' ? 'border-b-2 border-primary-500 text-primary-500' : 'text-slate-400 hover:text-slate-800'}`}
            onClick={() => setActiveTab('tickets')}
          >
            Tickets ({tickets.length})
          </button>
          <button
            className={`px-5 py-3 text-sm font-medium transition-colors ${activeTab === 'historique' ? 'border-b-2 border-primary-500 text-primary-500' : 'text-slate-400 hover:text-slate-800'}`}
            onClick={() => setActiveTab('historique')}
          >
            Historique
          </button>
        </div>

        {/* Tickets tab */}
        {activeTab === 'tickets' && (
          loading ? (
            <div className="py-12 text-center text-sm text-slate-400">Chargement des tickets...</div>
          ) : (
            <div
              ref={tableContainerRef}
              className="overflow-auto relative rounded-2xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
              style={{ height: '600px' }}
            >
              <table
                style={{ display: 'grid' }}
                aria-rowcount={tickets.length}
              >
                <thead style={{ display: 'grid', position: 'sticky', top: 0, zIndex: 1 }}
                  className="bg-slate-50">
                  {table.getHeaderGroups().map((hg) => (
                    <tr key={hg.id} style={{ display: 'flex', width: '100%' }}>
                      {hg.headers.map((header) => (
                        <th
                          key={header.id}
                          style={{ display: 'flex', width: header.getSize() }}
                          className="px-6 py-3.5 text-left text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans] cursor-pointer select-none"
                          onClick={header.column.getToggleSortingHandler()}
                        >
                          {flexRender(header.column.columnDef.header, header.getContext())}
                          {header.column.getCanSort()
                            ? ({ asc: ' ↑', desc: ' ↓' }[header.column.getIsSorted() as string] ?? ' ↕')
                            : null}
                        </th>
                      ))}
                    </tr>
                  ))}
                </thead>
                <tbody
                  style={{
                    display: 'grid',
                    height: `${rowVirtualizer.getTotalSize()}px`,
                    position: 'relative',
                  }}
                >
                  {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                    const row = rows[virtualRow.index];
                    const isOver90 = (row.original.ancienneteJours ?? 0) > 90;
                    return (
                      <tr
                        key={row.id}
                        data-index={virtualRow.index}
                        ref={(node) => rowVirtualizer.measureElement(node)}
                        style={{
                          display: 'flex',
                          position: 'absolute',
                          transform: `translateY(${virtualRow.start}px)`,
                          width: '100%',
                        }}
                        className={`border-b border-slate-50 ${isOver90 ? 'bg-danger-50' : 'hover:bg-[rgba(12,65,154,0.04)]'} transition-colors duration-100`}
                      >
                        {row.getVisibleCells().map((cell) => (
                          <td
                            key={cell.id}
                            style={{ display: 'flex', width: cell.column.getSize(), alignItems: 'center' }}
                            className="px-6 py-2 text-sm text-slate-800 overflow-hidden"
                          >
                            <span className="truncate">
                              {flexRender(cell.column.columnDef.cell, cell.getContext())}
                            </span>
                          </td>
                        ))}
                      </tr>
                    );
                  })}
                  {rows.length === 0 && (
                    <tr style={{ display: 'flex', position: 'absolute', width: '100%' }}>
                      <td className="px-6 py-8 text-center text-slate-400 w-full">
                        Aucun ticket vivant pour ce technicien
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          )
        )}

        {/* Historique tab */}
        {activeTab === 'historique' && decodedTech && (
          <HistoriqueTab technicien={decodedTech} />
        )}
      </div>
    </div>
  );
}

export default TechnicianDetail;
