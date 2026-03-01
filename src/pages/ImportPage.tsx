import { useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FileText,
  Upload,
  CheckCircle,
  AlertTriangle,
  Clock,
  History,
} from "lucide-react";
import { useImport } from "../hooks/useImport";
import { useInvoke } from "../hooks/useInvoke";
import type { ImportHistory } from "../types/config";
import { useAppStore } from "../stores/appStore";
import { Card } from "../components/shared/Card";

function formatDuration(ms: number): string {
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(2)} s`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("fr-FR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

const PHASE_LABELS: Record<string, string> = {
  parsing: "Lecture du CSV...",
  normalizing: "Normalisation des donnees...",
  inserting: "Insertion en base...",
  indexing: "Indexation...",
};

function StatCard({
  label,
  value,
  colorClass = "text-slate-800",
}: {
  label: string;
  value: number;
  colorClass?: string;
}) {
  return (
    <div className="bg-slate-50 rounded-xl p-3 text-center">
      <p className={`text-2xl font-bold font-[DM_Sans] ${colorClass}`}>
        {value.toLocaleString("fr-FR")}
      </p>
      <p className="text-xs text-slate-400 mt-1">{label}</p>
    </div>
  );
}

export default function ImportPage() {
  const {
    isImporting,
    progress,
    phase,
    result,
    error,
    warnings,
    startImport,
    reset,
  } = useImport();
  const { data: history, execute: fetchHistory } = useInvoke<ImportHistory[]>();
  const setCurrentImportId = useAppStore((s) => s.setCurrentImportId);

  useEffect(() => {
    fetchHistory("get_import_history");
    // fetchHistory is stable (useCallback with [])
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (result) {
      setCurrentImportId(result.importId);
      fetchHistory("get_import_history");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [result]);

  async function handleBrowse() {
    const selected = await open({
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (typeof selected === "string") {
      reset();
      await startImport(selected);
    }
  }

  const hasResult = result !== null;
  const hasError = error !== null;
  const showProgress = isImporting || hasResult || (hasError && progress > 0);

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Import CSV GLPI
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Importez un fichier CSV pour alimenter le tableau de bord
        </p>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6 max-w-3xl">
        {/* File selection */}
        <div className="animate-fade-slide-up">
          <Card>
            <div className="space-y-4">
              <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 flex items-center gap-2">
                <FileText className="w-5 h-5 text-primary-500" />
                Importer un fichier
              </h2>

              <button
                onClick={handleBrowse}
                disabled={isImporting}
                className="w-full px-4 py-3 rounded-xl bg-primary-500 text-white font-semibold hover:bg-primary-600 transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)] hover:shadow-[0_10px_20px_rgba(0,0,0,0.10),0_3px_6px_rgba(0,0,0,0.06)] active:shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]"
              >
                <Upload className="w-5 h-5" />
                {isImporting ? "Import en cours..." : "Importer un fichier CSV"}
              </button>

              {hasResult && result && (
                <p className="text-sm text-slate-400 text-center">
                  Fichier importe avec succes. Cliquez a nouveau pour importer un nouveau fichier.
                </p>
              )}
            </div>
          </Card>
        </div>

        {/* Progress */}
        {showProgress && (
          <div className="animate-fade-slide-up animation-delay-150">
            <Card>
              <div className="space-y-3">
                <div className="flex justify-between items-center">
                  <span className="text-sm font-medium text-slate-600">
                    {phase ? (PHASE_LABELS[phase] ?? phase) : "Preparation..."}
                  </span>
                  <span className="text-sm font-bold font-[DM_Sans] text-primary-500">{progress}%</span>
                </div>
                <div className="h-3 bg-slate-100 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary-500 rounded-full transition-all duration-300"
                    style={{ width: `${progress}%` }}
                  />
                </div>
              </div>
            </Card>
          </div>
        )}

        {/* Error */}
        {hasError && (
          <div className="bg-danger-50 rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-4 flex items-start gap-3 animate-fade-slide-up">
            <AlertTriangle className="w-5 h-5 text-danger-500 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-danger-500">Erreur d'import</p>
              <p className="text-sm text-red-600 mt-1">{error}</p>
            </div>
          </div>
        )}

        {/* Result */}
        {hasResult && result && (
          <div className="animate-fade-slide-up animation-delay-150">
            <Card>
              <div className="space-y-4">
                <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 flex items-center gap-2">
                  <CheckCircle className="w-5 h-5 text-success-500" />
                  Rapport d'import
                </h2>

                <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
                  <StatCard label="Total" value={result.totalTickets} />
                  <StatCard
                    label="Vivants"
                    value={result.vivantsCount}
                    colorClass="text-success-500"
                  />
                  <StatCard
                    label="Termines"
                    value={result.terminesCount}
                    colorClass="text-slate-500"
                  />
                  <StatCard
                    label="Ignores"
                    value={result.skippedRows}
                    colorClass="text-warning-500"
                  />
                </div>

                <div className="flex items-center gap-2 text-sm text-slate-600">
                  <Clock className="w-4 h-4" />
                  Duree :{" "}
                  <span className="font-medium">{formatDuration(result.parseDurationMs)}</span>
                </div>

                {warnings.length > 0 && (
                  <div>
                    <p className="text-sm font-semibold text-warning-500 mb-2">
                      {warnings.length} avertissement{warnings.length > 1 ? "s" : ""}
                    </p>
                    <ul className="space-y-1 max-h-40 overflow-y-auto text-sm text-slate-600 rounded-xl bg-slate-50 p-3">
                      {warnings.map((w, i) => (
                        <li key={i} className="flex gap-2">
                          <span className="text-slate-400 w-20 flex-shrink-0">
                            Ligne {w.line}
                          </span>
                          <span>{w.message}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            </Card>
          </div>
        )}

        {/* History */}
        <div className="animate-fade-slide-up animation-delay-300">
          <Card>
            <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700 flex items-center gap-2 mb-4">
              <History className="w-5 h-5 text-primary-500" />
              Historique des imports
            </h2>

            {!history || history.length === 0 ? (
              <p className="text-sm text-slate-400 italic">Aucun import enregistre.</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-100 text-left">
                      <th className="pb-2 pr-4 text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">Fichier</th>
                      <th className="pb-2 pr-4 text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">Date</th>
                      <th className="pb-2 pr-4 text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans] text-right">Tickets</th>
                      <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">Statut</th>
                    </tr>
                  </thead>
                  <tbody>
                    {history.map((h) => (
                      <tr key={h.id} className="border-b border-slate-50 last:border-0 hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100">
                        <td className="py-2 pr-4 text-slate-700 font-medium truncate max-w-[200px]">
                          {h.filename}
                        </td>
                        <td className="py-2 pr-4 text-slate-500 whitespace-nowrap">
                          {formatDate(h.importDate)}
                        </td>
                        <td className="py-2 pr-4 text-right text-slate-700 font-semibold font-[DM_Sans] tabular-nums">
                          {h.totalRows.toLocaleString("fr-FR")}
                        </td>
                        <td className="py-2">
                          {h.isActive ? (
                            <span className="inline-flex items-center px-2 py-0.5 rounded-lg text-xs font-medium bg-success-50 text-success-500">
                              Actif
                            </span>
                          ) : (
                            <span className="inline-flex items-center px-2 py-0.5 rounded-lg text-xs font-medium bg-slate-100 text-slate-500">
                              Archive
                            </span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </Card>
        </div>
      </div>
    </div>
  );
}
