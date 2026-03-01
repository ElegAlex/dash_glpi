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
  normalizing: "Normalisation des données...",
  inserting: "Insertion en base...",
  indexing: "Indexation...",
};

function StatCard({
  label,
  value,
  colorClass = "text-gray-900",
}: {
  label: string;
  value: number;
  colorClass?: string;
}) {
  return (
    <div className="bg-gray-50 rounded-lg p-3 text-center">
      <p className={`text-2xl font-bold ${colorClass}`}>
        {value.toLocaleString("fr-FR")}
      </p>
      <p className="text-xs text-gray-500 mt-1">{label}</p>
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
    <div className="space-y-6 p-6 max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold text-cpam-primary">Import CSV GLPI</h1>

      {/* File selection */}
      <div className="bg-white rounded-xl border border-gray-200 p-6 space-y-4">
        <h2 className="text-lg font-semibold text-gray-800 flex items-center gap-2">
          <FileText className="w-5 h-5 text-cpam-primary" />
          Importer un fichier
        </h2>

        <button
          onClick={handleBrowse}
          disabled={isImporting}
          className="w-full px-4 py-3 rounded-lg bg-cpam-primary text-white font-semibold hover:bg-cpam-primary-dark transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Upload className="w-5 h-5" />
          {isImporting ? "Import en cours..." : "Importer un fichier CSV"}
        </button>

        {hasResult && result && (
          <p className="text-sm text-gray-500 text-center">
            Fichier importé avec succès. Cliquez à nouveau pour importer un nouveau fichier.
          </p>
        )}
      </div>

      {/* Progress */}
      {showProgress && (
        <div className="bg-white rounded-xl border border-gray-200 p-6 space-y-3">
          <div className="flex justify-between items-center">
            <span className="text-sm font-medium text-gray-600">
              {phase ? (PHASE_LABELS[phase] ?? phase) : "Préparation..."}
            </span>
            <span className="text-sm font-bold text-cpam-primary">{progress}%</span>
          </div>
          <div className="h-3 bg-gray-100 rounded-full overflow-hidden">
            <div
              className="h-full bg-cpam-primary rounded-full transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      )}

      {/* Error */}
      {hasError && (
        <div className="bg-red-50 border border-red-200 rounded-xl p-4 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-danger flex-shrink-0 mt-0.5" />
          <div>
            <p className="font-semibold text-danger">Erreur d'import</p>
            <p className="text-sm text-red-600 mt-1">{error}</p>
          </div>
        </div>
      )}

      {/* Result */}
      {hasResult && result && (
        <div className="bg-white rounded-xl border border-gray-200 p-6 space-y-4">
          <h2 className="text-lg font-semibold text-gray-800 flex items-center gap-2">
            <CheckCircle className="w-5 h-5 text-success" />
            Rapport d'import
          </h2>

          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
            <StatCard label="Total" value={result.totalTickets} />
            <StatCard
              label="Vivants"
              value={result.vivantsCount}
              colorClass="text-success"
            />
            <StatCard
              label="Terminés"
              value={result.terminesCount}
              colorClass="text-gray-500"
            />
            <StatCard
              label="Ignorés"
              value={result.skippedRows}
              colorClass="text-warning"
            />
          </div>

          <div className="flex items-center gap-2 text-sm text-gray-600">
            <Clock className="w-4 h-4" />
            Durée :{" "}
            <span className="font-medium">{formatDuration(result.parseDurationMs)}</span>
          </div>

          {warnings.length > 0 && (
            <div>
              <p className="text-sm font-semibold text-warning mb-2">
                {warnings.length} avertissement{warnings.length > 1 ? "s" : ""}
              </p>
              <ul className="space-y-1 max-h-40 overflow-y-auto text-sm text-gray-600 border border-gray-100 rounded-lg p-3">
                {warnings.map((w, i) => (
                  <li key={i} className="flex gap-2">
                    <span className="text-gray-400 w-20 flex-shrink-0">
                      Ligne {w.line}
                    </span>
                    <span>{w.message}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* History */}
      <div className="bg-white rounded-xl border border-gray-200 p-6">
        <h2 className="text-lg font-semibold text-gray-800 flex items-center gap-2 mb-4">
          <History className="w-5 h-5 text-cpam-primary" />
          Historique des imports
        </h2>

        {!history || history.length === 0 ? (
          <p className="text-sm text-gray-400 italic">Aucun import enregistré.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-100 text-left text-gray-500">
                  <th className="pb-2 pr-4 font-medium">Fichier</th>
                  <th className="pb-2 pr-4 font-medium">Date</th>
                  <th className="pb-2 pr-4 font-medium text-right">Tickets</th>
                  <th className="pb-2 font-medium">Statut</th>
                </tr>
              </thead>
              <tbody>
                {history.map((h) => (
                  <tr key={h.id} className="border-b border-gray-50 last:border-0">
                    <td className="py-2 pr-4 text-gray-700 font-medium truncate max-w-[200px]">
                      {h.filename}
                    </td>
                    <td className="py-2 pr-4 text-gray-500 whitespace-nowrap">
                      {formatDate(h.importDate)}
                    </td>
                    <td className="py-2 pr-4 text-right text-gray-700">
                      {h.totalRows.toLocaleString("fr-FR")}
                    </td>
                    <td className="py-2">
                      {h.isActive ? (
                        <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700">
                          Actif
                        </span>
                      ) : (
                        <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-500">
                          Archivé
                        </span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
