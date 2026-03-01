import { useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import type { ExportResult } from '../../types/config';

interface CardState {
  loading: boolean;
  result: ExportResult | null;
  error: string | null;
}

const IDLE: CardState = { loading: false, result: null, error: null };

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} Mo`;
}

function formatDuration(ms: number): string {
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`;
}

async function runExport(
  command: string,
  args: Record<string, unknown>,
  setState: React.Dispatch<React.SetStateAction<CardState>>,
) {
  setState({ loading: true, result: null, error: null });
  try {
    const result = (await invoke(command, args)) as ExportResult;
    setState({ loading: false, result, error: null });
  } catch (err) {
    setState({ loading: false, result: null, error: String(err) });
  }
}

export function ExportPanel() {
  const [stockState, setStockState] = useState<CardState>(IDLE);
  const [planState, setPlanState] = useState<CardState>(IDLE);
  const [bilanState, setBilanState] = useState<CardState>(IDLE);
  const [zipState, setZipState] = useState<CardState>(IDLE);
  const [technicien, setTechnicien] = useState('');

  const handleStock = async () => {
    const path = await save({
      defaultPath: 'stock_dashboard.xlsx',
      filters: [{ name: 'Excel', extensions: ['xlsx'] }],
    });
    if (!path) return;
    await runExport('export_excel_stock', { path }, setStockState);
  };

  const handlePlan = async () => {
    const name = technicien.trim();
    if (!name) return;
    const safeName = name.replace(/\s+/g, '_');
    const path = await save({
      defaultPath: `plan_action_${safeName}.xlsx`,
      filters: [{ name: 'Excel', extensions: ['xlsx'] }],
    });
    if (!path) return;
    await runExport('export_excel_plan_action', { technicien: name, path }, setPlanState);
  };

  const handleBilan = async () => {
    const path = await save({
      defaultPath: 'bilan.xlsx',
      filters: [{ name: 'Excel', extensions: ['xlsx'] }],
    });
    if (!path) return;
    await runExport('export_excel_bilan', { path }, setBilanState);
  };

  const handleZip = async () => {
    const path = await save({
      defaultPath: 'plans_action.zip',
      filters: [{ name: 'ZIP', extensions: ['zip'] }],
    });
    if (!path) return;
    await runExport('export_all_plans_zip', { path }, setZipState);
  };

  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
      <ExportCard
        title="Stock Dashboard"
        description="Vue globale, techniciens et groupes — 3 onglets XLSX."
        state={stockState}
        onExport={handleStock}
        buttonLabel="Exporter le stock"
      />

      <ExportCard
        title="Plan d'action technicien"
        description="Entretien, tickets et checklist — XLSX individuel."
        state={planState}
        onExport={handlePlan}
        buttonLabel="Exporter"
        disabled={!technicien.trim()}
        extra={
          <input
            type="text"
            placeholder="Nom du technicien"
            value={technicien}
            onChange={(e) => setTechnicien(e.target.value)}
            className="w-full rounded-md border border-[#cdd3df] px-3 py-1.5 text-sm text-[#1a1f2e] placeholder-[#6e7891] focus:outline-none focus:ring-2 focus:ring-[#0C419A] focus:border-transparent"
          />
        }
      />

      <ExportCard
        title="Bilan d'activité"
        description="Flux entrants/sortants, délais et comparatif techniciens."
        state={bilanState}
        onExport={handleBilan}
        buttonLabel="Exporter le bilan"
      />

      <ExportCard
        title="Tous les plans d'action (ZIP)"
        description="Archive contenant un XLSX par technicien avec tickets vivants."
        state={zipState}
        onExport={handleZip}
        buttonLabel="Exporter le ZIP"
      />
    </div>
  );
}

interface ExportCardProps {
  title: string;
  description: string;
  state: CardState;
  onExport: () => void;
  buttonLabel: string;
  disabled?: boolean;
  extra?: React.ReactNode;
}

function ExportCard({
  title,
  description,
  state,
  onExport,
  buttonLabel,
  disabled,
  extra,
}: ExportCardProps) {
  return (
    <div className="rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)] p-5 flex flex-col gap-3">
      <div>
        <h3 className="text-sm font-semibold text-[#1a1f2e]">{title}</h3>
        <p className="mt-0.5 text-xs text-[#6e7891]">{description}</p>
      </div>

      {extra && <div>{extra}</div>}

      <button
        onClick={onExport}
        disabled={state.loading || disabled}
        className="w-full rounded-md bg-[#0C419A] px-3 py-2 text-sm font-medium text-white hover:bg-[#0a3783] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {state.loading ? 'Génération en cours…' : buttonLabel}
      </button>

      {state.error && (
        <div className="rounded-md bg-[#fef2f2] border border-[#ce0500] px-3 py-2 text-xs text-[#af0400]">
          {state.error}
        </div>
      )}

      {state.result && (
        <div className="rounded-md bg-[#f0faf4] border border-[#009E73] px-3 py-2 text-xs text-[#18753c]">
          <div className="font-medium truncate" title={state.result.path}>
            {state.result.path}
          </div>
          <div className="mt-0.5 text-[#525d73]">
            {formatSize(state.result.sizeBytes)} &middot; {formatDuration(state.result.durationMs)}
          </div>
        </div>
      )}
    </div>
  );
}
