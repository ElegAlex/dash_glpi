import { useMemo } from 'react';
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  getPaginationRowModel,
  useReactTable,
  type SortingState,
} from '@tanstack/react-table';
import { useState } from 'react';
import { X, ChevronUp, ChevronDown, ChevronLeft, ChevronRight } from 'lucide-react';
import { useECharts } from '../../hooks/useECharts';
import { Card } from '../shared/Card';
import type { ClusterDetail, ClusterTicket, ClusterInfo } from '../../types/mining';
import '../../lib/echarts-theme';

const PALETTE = ['#1565C0', '#2E7D32', '#FF8F00', '#6A1B9A', '#00838F', '#C62828', '#4E342E', '#37474F'];

// ── Sub-charts ──────────────────────────────────────────────────────────────

function HorizontalBarChart({ data, title }: { data: { label: string; count: number }[]; title: string }) {
  const option = useMemo(() => {
    const top10 = data.slice(0, 10);
    return {
      tooltip: { trigger: 'axis' as const, axisPointer: { type: 'shadow' as const } },
      grid: { left: 130, right: 30, top: 10, bottom: 20 },
      xAxis: { type: 'value' as const },
      yAxis: {
        type: 'category' as const,
        data: top10.map((d) => d.label),
        inverse: true,
        axisLabel: { fontSize: 11, width: 115, overflow: 'truncate' as const },
      },
      series: [{
        type: 'bar' as const,
        data: top10.map((d, i) => ({
          value: d.count,
          itemStyle: { color: PALETTE[i % PALETTE.length], borderRadius: [0, 6, 6, 0] },
        })),
        barWidth: '60%',
        label: {
          show: true,
          position: 'right' as const,
          fontSize: 11,
          color: '#64748B',
          formatter: (p: { value: number }) => p.value.toLocaleString('fr-FR'),
        },
      }],
    };
  }, [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');
  if (data.length === 0) return <p className="text-sm text-slate-400 py-4 text-center">Aucune donnee</p>;
  return (
    <div>
      <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 mb-2">{title}</h3>
      <div ref={chartRef} style={{ height: 240, width: '100%' }} />
    </div>
  );
}

function EvolutionLineChart({ data }: { data: { periode: string; count: number }[] }) {
  const option = useMemo(() => ({
    tooltip: { trigger: 'axis' as const },
    grid: { left: 45, right: 15, top: 15, bottom: 25 },
    xAxis: {
      type: 'category' as const,
      data: data.map((d) => d.periode),
      axisLabel: { fontSize: 10, rotate: data.length > 8 ? 45 : 0 },
    },
    yAxis: {
      type: 'value' as const,
      splitLine: { lineStyle: { type: 'dashed' as const, color: '#F1F5F9' } },
    },
    series: [{
      type: 'line' as const,
      data: data.map((d) => d.count),
      smooth: true,
      lineStyle: { width: 3, color: '#1565C0' },
      itemStyle: { color: '#1565C0', borderColor: '#fff', borderWidth: 2 },
      areaStyle: {
        color: {
          type: 'linear' as const,
          x: 0, y: 0, x2: 0, y2: 1,
          colorStops: [
            { offset: 0, color: 'rgba(21,101,192,0.15)' },
            { offset: 1, color: 'rgba(21,101,192,0.02)' },
          ],
        },
      },
    }],
  }), [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');
  return <div ref={chartRef} style={{ height: 200, width: '100%' }} />;
}

// ── RAG badge ───────────────────────────────────────────────────────────────

function RagBadge({ value, label, danger }: { value: number; label: string; danger?: boolean }) {
  const bg = value === 0
    ? 'bg-emerald-50 text-emerald-700'
    : danger
      ? 'bg-red-50 text-red-700'
      : 'bg-amber-50 text-amber-800';
  return (
    <div className={`rounded-2xl p-4 ${bg}`}>
      <div className="text-2xl font-bold font-[DM_Sans] tabular-nums">{value}</div>
      <div className="text-xs font-medium mt-1">{label}</div>
    </div>
  );
}

// ── KPI mini card ───────────────────────────────────────────────────────────

function MiniKpi({ label, value, suffix, comparison }: {
  label: string;
  value: string | number | null;
  suffix?: string;
  comparison?: { global: number | null; label: string };
}) {
  const display = value == null ? '—' : typeof value === 'number' ? value.toFixed(1) : value;
  const delta = comparison && comparison.global != null && typeof value === 'number'
    ? ((value - comparison.global) / comparison.global * 100)
    : null;

  return (
    <div className="bg-white rounded-2xl p-4 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
      <div className="text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans] mb-1">{label}</div>
      <div className="flex items-baseline gap-2">
        <span className="text-2xl font-bold font-[DM_Sans] tabular-nums tracking-tight text-slate-800">
          {display}{suffix && <span className="text-sm font-normal text-slate-400 ml-0.5">{suffix}</span>}
        </span>
        {delta != null && (
          <span className={`text-xs font-semibold px-2 py-0.5 rounded-lg ${
            delta <= 0 ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'
          }`}>
            {delta > 0 ? '+' : ''}{delta.toFixed(0)}%
          </span>
        )}
      </div>
      {comparison && comparison.global != null && (
        <div className="text-xs text-slate-400 mt-1">{comparison.label} : {comparison.global.toFixed(1)}j</div>
      )}
    </div>
  );
}

// ── Ticket table ────────────────────────────────────────────────────────────

const columnHelper = createColumnHelper<ClusterTicket>();

function ageColor(days: number | null): string {
  if (days == null) return 'text-slate-400';
  if (days <= 10) return 'text-emerald-600';
  if (days <= 20) return 'text-amber-600';
  if (days <= 40) return 'text-orange-600';
  return 'text-red-600';
}

function statusBadge(statut: string) {
  const s = statut.toLowerCase();
  if (s.includes('clos') || s.includes('résolu'))
    return 'bg-emerald-50 text-emerald-700';
  if (s.includes('attente'))
    return 'bg-amber-50 text-amber-800';
  return 'bg-blue-50 text-blue-700';
}

const columns = [
  columnHelper.accessor('id', {
    header: 'ID',
    size: 70,
    cell: (info) => <span className="font-[DM_Sans] font-semibold text-primary-500">#{info.getValue()}</span>,
  }),
  columnHelper.accessor('titre', {
    header: 'Titre',
    size: 300,
    cell: (info) => (
      <span className="block truncate max-w-[300px]" title={info.getValue()}>
        {info.getValue()}
      </span>
    ),
  }),
  columnHelper.accessor('statut', {
    header: 'Statut',
    size: 130,
    cell: (info) => (
      <span className={`rounded-lg px-2 py-0.5 text-xs font-medium ${statusBadge(info.getValue())}`}>
        {info.getValue()}
      </span>
    ),
  }),
  columnHelper.accessor('technicien', {
    header: 'Technicien',
    size: 140,
    cell: (info) => info.getValue() ?? <span className="text-slate-300">—</span>,
  }),
  columnHelper.accessor('ancienneteJours', {
    header: 'Age',
    size: 70,
    cell: (info) => {
      const v = info.getValue();
      return <span className={`font-[DM_Sans] font-semibold tabular-nums ${ageColor(v)}`}>{v ?? '—'}j</span>;
    },
  }),
  columnHelper.accessor('nbSuivis', {
    header: 'Suivis',
    size: 60,
    cell: (info) => <span className="font-[DM_Sans] tabular-nums">{info.getValue()}</span>,
  }),
];

function TicketTable({ tickets }: { tickets: ClusterTicket[] }) {
  const [sorting, setSorting] = useState<SortingState>([]);

  const table = useReactTable({
    data: tickets,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    initialState: { pagination: { pageSize: 20 } },
  });

  return (
    <div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            {table.getHeaderGroups().map((hg) => (
              <tr key={hg.id}>
                {hg.headers.map((h) => (
                  <th
                    key={h.id}
                    onClick={h.column.getToggleSortingHandler()}
                    className="text-left px-3 py-2 text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans] cursor-pointer select-none"
                    style={{ width: h.getSize() }}
                  >
                    <div className="flex items-center gap-1">
                      {flexRender(h.column.columnDef.header, h.getContext())}
                      {h.column.getIsSorted() === 'asc' && <ChevronUp size={12} />}
                      {h.column.getIsSorted() === 'desc' && <ChevronDown size={12} />}
                    </div>
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr key={row.id} className="hover:bg-[rgba(12,65,154,0.04)] transition-colors">
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="px-3 py-2 text-slate-800">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {table.getPageCount() > 1 && (
        <div className="flex items-center justify-between px-3 py-3 border-t border-slate-100 text-xs text-slate-400">
          <span>
            Page {table.getState().pagination.pageIndex + 1} / {table.getPageCount()} ({tickets.length} tickets)
          </span>
          <div className="flex gap-1">
            <button
              onClick={() => table.previousPage()}
              disabled={!table.getCanPreviousPage()}
              className="p-1 rounded hover:bg-slate-100 disabled:opacity-30"
            >
              <ChevronLeft size={16} />
            </button>
            <button
              onClick={() => table.nextPage()}
              disabled={!table.getCanNextPage()}
              className="p-1 rounded hover:bg-slate-100 disabled:opacity-30"
            >
              <ChevronRight size={16} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Main panel ──────────────────────────────────────────────────────────────

interface ClusterDetailPanelProps {
  cluster: ClusterInfo;
  detail: ClusterDetail;
  loading: boolean;
  error: string | null;
  onClose: () => void;
}

export default function ClusterDetailPanel({ cluster, detail, loading, error, onClose }: ClusterDetailPanelProps) {
  const color = PALETTE[cluster.id % PALETTE.length];

  return (
    <>
      {/* Overlay */}
      <div className="fixed inset-0 z-40 bg-black/20" onClick={onClose} />

      {/* Drawer */}
      <div className="fixed inset-y-0 right-0 z-50 w-[65%] min-w-[600px] max-w-[1100px] bg-white shadow-[0_15px_30px_rgba(0,0,0,0.12),0_5px_15px_rgba(0,0,0,0.08)] rounded-l-2xl flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-100 shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-3 h-3 rounded-full" style={{ background: color }} />
            <div>
              <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-800">
                Cluster {cluster.id + 1}
              </h2>
              <p className="text-xs text-slate-400 mt-0.5">
                {cluster.label || cluster.topKeywords.slice(0, 3).join(' · ')} — {cluster.ticketCount} tickets
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-600 transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-5 space-y-6">
          {loading && (
            <div className="flex flex-col items-center justify-center py-16 gap-3 text-slate-400">
              <div className="h-8 w-8 animate-spin rounded-full border-2 border-primary-500 border-t-transparent" />
              <span className="text-sm">Chargement du detail...</span>
            </div>
          )}

          {error && (
            <div className="rounded-2xl bg-red-50 px-4 py-3 text-sm text-red-600">{error}</div>
          )}

          {!loading && !error && detail && (
            <>
              {/* 1. Profil */}
              <section>
                <h3 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-3">Profil du cluster</h3>
                <div className="grid grid-cols-2 sm:grid-cols-3 xl:grid-cols-6 gap-3">
                  <MiniKpi
                    label="MTTR moyen"
                    value={detail.mttrAvg}
                    suffix="j"
                    comparison={{ global: detail.globalMttrAvg, label: 'Global' }}
                  />
                  <MiniKpi label="MTTR median" value={detail.mttrMedian} suffix="j" />
                  <MiniKpi label="Ratio Inc/Dem" value={detail.ratioIncidentsDemandes.toFixed(2)} />
                  <MiniKpi label="Vivants" value={detail.nbVivants} />
                  <MiniKpi label="Moy. suivis" value={detail.avgSuivis.toFixed(1)} />
                  <MiniKpi
                    label="Anc. moy. vivants"
                    value={detail.ancienneteAvgVivants}
                    suffix="j"
                  />
                </div>
              </section>

              {/* 2. Répartition */}
              <section>
                <h3 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-3">Repartition</h3>
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
                  <Card>
                    <HorizontalBarChart data={detail.parTechnicien} title="Top techniciens" />
                  </Card>
                  <Card>
                    <HorizontalBarChart data={detail.parGroupe} title="Groupes" />
                  </Card>
                </div>
              </section>

              {/* 3. Stock vivant */}
              <section>
                <h3 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-3">Stock vivant</h3>
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                  <RagBadge value={detail.stockVivants} label="Vivants" />
                  <RagBadge value={detail.stockSansSuivi} label="Sans suivi" danger />
                  <RagBadge value={detail.stockPlus90j} label="> 90 jours" danger />
                  <RagBadge value={detail.stockInactifs14j} label="Inactifs > 14j" />
                </div>
              </section>

              {/* 4. Évolution */}
              {detail.evolutionMensuelle.length > 0 && (
                <section>
                  <h3 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-3">Evolution temporelle</h3>
                  <Card>
                    <EvolutionLineChart data={detail.evolutionMensuelle} />
                  </Card>
                </section>
              )}

              {/* 5. Liste tickets */}
              <section>
                <h3 className="text-lg font-semibold font-[DM_Sans] text-slate-700 mb-3">
                  Tickets du cluster ({detail.tickets.length})
                </h3>
                <Card className="overflow-hidden">
                  <TicketTable tickets={detail.tickets} />
                </Card>
              </section>
            </>
          )}
        </div>
      </div>
    </>
  );
}
