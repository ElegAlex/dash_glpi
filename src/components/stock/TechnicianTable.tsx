import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  createColumnHelper,
  type SortingState,
} from '@tanstack/react-table';
import type { TechnicianStats } from '../../types/kpi';

type RagLevel = 'Vert' | 'Jaune' | 'Orange' | 'Rouge';

function ragLevel(total: number): RagLevel {
  if (total < 10) return 'Vert';
  if (total < 20) return 'Jaune';
  if (total <= 40) return 'Orange';
  return 'Rouge';
}

const RAG_STYLES: Record<RagLevel, string> = {
  Vert:   'bg-success-50 text-success-500',
  Jaune:  'bg-accent-50 text-accent-700',
  Orange: 'bg-warning-50 text-warning-500',
  Rouge:  'bg-danger-50 text-danger-500',
};

const columnHelper = createColumnHelper<TechnicianStats>();

interface TechnicianTableProps {
  data: TechnicianStats[];
}

export function TechnicianTable({ data }: TechnicianTableProps) {
  const navigate = useNavigate();
  const [sorting, setSorting] = useState<SortingState>([{ id: 'total', desc: true }]);

  const columns = useMemo(
    () => [
      columnHelper.accessor('technicien', {
        header: 'Technicien',
        cell: (info) => (
          <button
            className="text-primary-500 hover:underline font-medium text-left"
            onClick={() => navigate(`/stock/${encodeURIComponent(info.getValue())}`)}
          >
            {info.getValue()}
          </button>
        ),
      }),
      columnHelper.accessor('total', {
        header: 'Stock',
        cell: (info) => info.getValue().toLocaleString('fr-FR'),
      }),
      columnHelper.accessor('enCours', {
        header: 'En cours',
        cell: (info) => info.getValue(),
      }),
      columnHelper.accessor('enAttente', {
        header: 'En attente',
        cell: (info) => info.getValue(),
      }),
      columnHelper.accessor('incidents', {
        header: 'Incidents',
        cell: (info) => info.getValue(),
      }),
      columnHelper.accessor('demandes', {
        header: 'Demandes',
        cell: (info) => info.getValue(),
      }),
      columnHelper.display({
        id: 'over90',
        header: '> 90 j',
        cell: ({ row }) => {
          const val = row.original.ecartSeuil > 0 ? row.original.ecartSeuil : 0;
          return val > 0 ? (
            <span className="text-danger-500 font-medium">{val}</span>
          ) : (
            <span className="text-slate-400">0</span>
          );
        },
      }),
      columnHelper.display({
        id: 'rag',
        header: 'RAG',
        cell: ({ row }) => {
          const level = ragLevel(row.original.total);
          return (
            <span className={`inline-flex items-center rounded-lg px-2 py-0.5 text-xs font-medium ${RAG_STYLES[level]}`}>
              {level}
            </span>
          );
        },
      }),
    ],
    [navigate],
  );

  const table = useReactTable({
    data,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return (
    <div className="overflow-auto rounded-2xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
      <table className="w-full text-sm">
        <thead className="bg-slate-50 sticky top-0 z-10">
          {table.getHeaderGroups().map((hg) => (
            <tr key={hg.id}>
              {hg.headers.map((header) => (
                <th
                  key={header.id}
                  className="px-6 py-3.5 text-left text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans] whitespace-nowrap cursor-pointer select-none"
                  onClick={header.column.getToggleSortingHandler()}
                >
                  {flexRender(header.column.columnDef.header, header.getContext())}
                  {header.column.getCanSort()
                    ? ({ asc: ' \u2191', desc: ' \u2193' }[header.column.getIsSorted() as string] ?? ' \u2195')
                    : null}
                </th>
              ))}
            </tr>
          ))}
        </thead>
        <tbody>
          {table.getRowModel().rows.map((row) => (
            <tr key={row.id} className="border-b border-slate-50 last:border-0 hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100">
              {row.getVisibleCells().map((cell) => (
                <td key={cell.id} className="px-6 py-3 text-slate-800">
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </td>
              ))}
            </tr>
          ))}
          {table.getRowModel().rows.length === 0 && (
            <tr>
              <td colSpan={columns.length} className="px-6 py-8 text-center text-slate-400">
                Aucun technicien
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
