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
import type { ExportResult, TechTimelinePoint } from '../types/config';
import type { EChartsCoreOption } from 'echarts/core';

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

function HistoriqueTab({ technicien }: { technicien: string }) {
  const { data, loading, error, execute } = useInvoke<TechTimelinePoint[]>();

  useEffect(() => {
    execute('get_technician_timeline', { technicien });
  }, [technicien]); // eslint-disable-line react-hooks/exhaustive-deps

  const timeline = useMemo(() => data ?? [], [data]);

  const chartOption = useMemo<EChartsCoreOption>(() => {
    if (timeline.length < 2) return {};
    const dates = timeline.map((p) => new Date(p.importDate).toLocaleDateString('fr-FR'));
    return {
      tooltip: { trigger: 'axis', axisPointer: { type: 'cross' } },
      legend: { data: ['Tickets', 'Age moyen (j)'], bottom: 0 },
      grid: { left: 60, right: 60, top: 20, bottom: 50 },
      xAxis: { type: 'category', data: dates },
      yAxis: [
        { type: 'value', name: 'Tickets', position: 'left' },
        { type: 'value', name: 'Age (j)', position: 'right' },
      ],
      series: [
        {
          name: 'Tickets',
          type: 'bar',
          yAxisIndex: 0,
          data: timeline.map((p) => p.ticketCount),
          itemStyle: { color: '#0C419A' },
        },
        {
          name: 'Age moyen (j)',
          type: 'line',
          yAxisIndex: 1,
          data: timeline.map((p) => Math.round(p.avgAge)),
          itemStyle: { color: '#FF8F00' },
          lineStyle: { color: '#FF8F00' },
          symbol: 'circle',
          symbolSize: 6,
        },
      ],
    };
  }, [timeline]);

  const { chartRef } = useECharts(timeline.length >= 2 ? chartOption : {}, undefined, 'cpam-material');

  if (loading) {
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

  if (timeline.length < 2) {
    return (
      <div className="py-12 text-center text-sm text-slate-400">
        Donnees insuffisantes pour l'historique (au moins 2 imports requis)
      </div>
    );
  }

  return (
    <div className="space-y-4 pt-4">
      {/* Chart */}
      <div
        ref={chartRef}
        className="rounded-2xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
        style={{ height: 320 }}
      />

      {/* Summary table */}
      <div className="rounded-2xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] overflow-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-100">
              <th className="px-6 py-3.5 text-left text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
                Date import
              </th>
              <th className="px-6 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
                Tickets
              </th>
              <th className="px-6 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
                Age moyen (j)
              </th>
            </tr>
          </thead>
          <tbody>
            {timeline.map((pt) => (
              <tr key={pt.importId} className="border-b border-slate-50 last:border-0 hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100">
                <td className="px-6 py-3.5 text-slate-800">
                  {new Date(pt.importDate).toLocaleDateString('fr-FR')}
                </td>
                <td className="px-6 py-3.5 text-right text-slate-800 font-semibold font-[DM_Sans] tabular-nums">
                  {pt.ticketCount.toLocaleString('fr-FR')}
                </td>
                <td className="px-6 py-3.5 text-right text-slate-800 font-semibold font-[DM_Sans] tabular-nums">
                  {Math.round(pt.avgAge)}
                </td>
              </tr>
            ))}
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
