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
    header: 'Priorité',
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
    header: 'Âge (j)',
    size: 80,
    cell: (info) => {
      const v = info.getValue();
      if (v === null || v === undefined) return '—';
      const cls = v > 90 ? 'text-[#af0400] font-medium' : v > 30 ? 'text-[#d4600a]' : 'text-[#1a1f2e]';
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
      legend: { data: ['Tickets', 'Âge moyen (j)'], bottom: 0 },
      grid: { left: 60, right: 60, top: 20, bottom: 50 },
      xAxis: { type: 'category', data: dates },
      yAxis: [
        { type: 'value', name: 'Tickets', position: 'left' },
        { type: 'value', name: 'Âge (j)', position: 'right' },
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
          name: 'Âge moyen (j)',
          type: 'line',
          yAxisIndex: 1,
          data: timeline.map((p) => Math.round(p.avgAge)),
          itemStyle: { color: '#D55E00' },
          lineStyle: { color: '#D55E00' },
          symbol: 'circle',
          symbolSize: 6,
        },
      ],
    };
  }, [timeline]);

  const { chartRef } = useECharts(timeline.length >= 2 ? chartOption : {});

  if (loading) {
    return (
      <div className="py-12 text-center text-sm text-[#6e7891]">Chargement de l'historique…</div>
    );
  }

  if (error) {
    return (
      <div className="rounded-md bg-[#fef2f2] border border-[#ce0500] px-4 py-3 text-sm text-[#af0400]">
        {error}
      </div>
    );
  }

  if (timeline.length < 2) {
    return (
      <div className="py-12 text-center text-sm text-[#6e7891]">
        Données insuffisantes pour l'historique (au moins 2 imports requis)
      </div>
    );
  }

  return (
    <div className="space-y-4 pt-4">
      {/* Chart */}
      <div
        ref={chartRef}
        className="rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)]"
        style={{ height: 320 }}
      />

      {/* Summary table */}
      <div className="rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)] overflow-auto">
        <table className="w-full text-sm">
          <thead className="bg-[#f1f3f7]">
            <tr>
              <th className="px-4 py-3 text-left text-xs font-medium text-[#525d73] uppercase tracking-wide">
                Date import
              </th>
              <th className="px-4 py-3 text-right text-xs font-medium text-[#525d73] uppercase tracking-wide">
                Tickets
              </th>
              <th className="px-4 py-3 text-right text-xs font-medium text-[#525d73] uppercase tracking-wide">
                Âge moyen (j)
              </th>
            </tr>
          </thead>
          <tbody>
            {timeline.map((pt) => (
              <tr key={pt.importId} className="border-t border-[#e2e6ee] hover:bg-[#f8f9fb] transition-colors">
                <td className="px-4 py-2 text-[#1a1f2e]">
                  {new Date(pt.importDate).toLocaleDateString('fr-FR')}
                </td>
                <td className="px-4 py-2 text-right text-[#1a1f2e]">
                  {pt.ticketCount.toLocaleString('fr-FR')}
                </td>
                <td className="px-4 py-2 text-right text-[#1a1f2e]">
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
      execute('get_technician_tickets', { technicien: decodedTech });
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
    <div className="space-y-4">
      {/* Breadcrumb */}
      <nav className="flex items-center gap-1 text-sm text-[#525d73]">
        <button
          onClick={() => navigate('/stock')}
          className="hover:text-[#0C419A] font-medium transition-colors"
        >
          Stock
        </button>
        <span className="text-[#6e7891]">/</span>
        <span className="font-medium text-[#1a1f2e]">{decodedTech || '…'}</span>
      </nav>

      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-[#1a1f2e]">
            Technicien : {decodedTech}
          </h1>
          {!loading && tickets.length > 0 && (
            <p className="mt-0.5 text-sm text-[#525d73]">
              {tickets.length.toLocaleString('fr-FR')} ticket(s) vivant(s)
              {over90Count > 0 && (
                <span className="ml-2 inline-flex items-center rounded-full bg-[#fef2f2] border border-[#ce0500] px-2 py-0.5 text-xs font-medium text-[#af0400]">
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
            className="rounded-md bg-[#0C419A] px-3 py-1.5 text-sm font-medium text-white hover:bg-[#0a3783] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {exporting ? 'Export...' : "Exporter le plan d'action"}
          </button>
          <button
            onClick={() => navigate('/stock')}
            className="rounded-md border border-[#cdd3df] bg-white px-3 py-1.5 text-sm text-[#525d73] hover:bg-[#f1f3f7] transition-colors"
          >
            ← Retour
          </button>
        </div>
      </div>

      {error && (
        <div className="rounded-md bg-[#fef2f2] border border-[#ce0500] px-4 py-3 text-sm text-[#af0400]">
          {error}
        </div>
      )}

      {/* Tab bar */}
      <div className="flex border-b border-[#e2e6ee]">
        <button
          className={`px-4 py-2 text-sm font-medium border-b-2 ${activeTab === 'tickets' ? 'border-[#0C419A] text-[#0C419A]' : 'border-transparent text-[#6e7891] hover:text-[#525d73]'}`}
          onClick={() => setActiveTab('tickets')}
        >
          Tickets ({tickets.length})
        </button>
        <button
          className={`px-4 py-2 text-sm font-medium border-b-2 ${activeTab === 'historique' ? 'border-[#0C419A] text-[#0C419A]' : 'border-transparent text-[#6e7891] hover:text-[#525d73]'}`}
          onClick={() => setActiveTab('historique')}
        >
          Historique
        </button>
      </div>

      {/* Tickets tab */}
      {activeTab === 'tickets' && (
        loading ? (
          <div className="py-12 text-center text-sm text-[#6e7891]">Chargement des tickets…</div>
        ) : (
          <div
            ref={tableContainerRef}
            className="overflow-auto relative rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)]"
            style={{ height: '600px' }}
          >
            <table
              style={{ display: 'grid' }}
              aria-rowcount={tickets.length}
            >
              <thead style={{ display: 'grid', position: 'sticky', top: 0, zIndex: 1 }}
                className="bg-[#f1f3f7]">
                {table.getHeaderGroups().map((hg) => (
                  <tr key={hg.id} style={{ display: 'flex', width: '100%' }}>
                    {hg.headers.map((header) => (
                      <th
                        key={header.id}
                        style={{ display: 'flex', width: header.getSize() }}
                        className="px-4 py-3 text-left text-xs font-medium text-[#525d73] uppercase tracking-wide cursor-pointer select-none"
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
                      className={`border-b border-[#e2e6ee] ${isOver90 ? 'bg-[#fef2f2]' : 'hover:bg-[#f8f9fb]'} transition-colors`}
                    >
                      {row.getVisibleCells().map((cell) => (
                        <td
                          key={cell.id}
                          style={{ display: 'flex', width: cell.column.getSize(), alignItems: 'center' }}
                          className="px-4 py-2 text-sm text-[#1a1f2e] overflow-hidden"
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
                    <td className="px-4 py-8 text-center text-[#6e7891] w-full">
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
  );
}

export default TechnicianDetail;
