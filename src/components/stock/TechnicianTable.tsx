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
  Vert:   'bg-[#f0faf4] text-[#136130] border border-[#18753c]',
  Jaune:  'bg-[#fef9ec] text-[#965e04] border border-[#b27806]',
  Orange: 'bg-orange-50 text-[#d4600a] border border-[#d4600a]',
  Rouge:  'bg-[#fef2f2] text-[#af0400] border border-[#ce0500]',
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
            className="text-[#0C419A] hover:underline font-medium text-left"
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
            <span className="text-[#af0400] font-medium">{val}</span>
          ) : (
            <span className="text-[#6e7891]">0</span>
          );
        },
      }),
      columnHelper.display({
        id: 'rag',
        header: 'RAG',
        cell: ({ row }) => {
          const level = ragLevel(row.original.total);
          return (
            <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${RAG_STYLES[level]}`}>
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
    <div className="overflow-auto rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)]">
      <table className="w-full text-sm">
        <thead className="bg-[#f1f3f7] sticky top-0 z-10">
          {table.getHeaderGroups().map((hg) => (
            <tr key={hg.id}>
              {hg.headers.map((header) => (
                <th
                  key={header.id}
                  className="px-4 py-3 text-left text-xs font-medium text-[#525d73] uppercase tracking-wide whitespace-nowrap cursor-pointer select-none"
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
        <tbody className="divide-y divide-[#e2e6ee]">
          {table.getRowModel().rows.map((row) => (
            <tr key={row.id} className="hover:bg-[#f8f9fb] transition-colors">
              {row.getVisibleCells().map((cell) => (
                <td key={cell.id} className="px-4 py-2 text-[#1a1f2e]">
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </td>
              ))}
            </tr>
          ))}
          {table.getRowModel().rows.length === 0 && (
            <tr>
              <td colSpan={columns.length} className="px-4 py-8 text-center text-[#6e7891]">
                Aucun technicien
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
