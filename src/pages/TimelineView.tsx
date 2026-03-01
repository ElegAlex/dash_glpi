import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useInvoke } from "../hooks/useInvoke";
import { useECharts } from "../hooks/useECharts";
import type { TimelinePoint, ImportComparison } from "../types/config";

function formatDate(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  return d.toLocaleDateString("fr-FR");
}

function DeltaCard({ label, value }: { label: string; value: number }) {
  const positive = value >= 0;
  return (
    <div
      className={`rounded-2xl p-4 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] ${
        positive ? "bg-success-50" : "bg-danger-50"
      }`}
    >
      <span className="block text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">{label}</span>
      <span
        className={`text-xl font-bold font-[DM_Sans] ${
          positive ? "text-success-500" : "text-danger-500"
        }`}
      >
        {positive ? "+" : ""}
        {value}
      </span>
    </div>
  );
}

function InfoCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-2xl bg-white p-4 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
      <span className="block text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">{label}</span>
      <span className="text-xl font-bold font-[DM_Sans] text-slate-800">{value}</span>
    </div>
  );
}

function TimelineChart({ points }: { points: TimelinePoint[] }) {
  const option = useMemo(() => {
    const labels = points.map((p) => formatDate(p.importDate));
    const vivants = points.map((p) => p.vivantsCount);
    const termines = points.map((p) => p.terminesCount);

    return {
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "cross" },
        formatter: (params: { dataIndex: number }[]) => {
          const idx = params[0]?.dataIndex ?? 0;
          const pt = points[idx];
          if (!pt) return "";
          return [
            `<strong>${pt.filename}</strong>`,
            `Date : ${formatDate(pt.importDate)}`,
            `Vivants : ${pt.vivantsCount}`,
            `Termines : ${pt.terminesCount}`,
            `Total : ${pt.totalRows}`,
          ].join("<br/>");
        },
      },
      legend: { data: ["Vivants", "Termines"], top: 8 },
      grid: { top: 56, bottom: 40, left: 56, right: 24 },
      xAxis: {
        type: "category",
        data: labels,
        axisLabel: { rotate: labels.length > 8 ? 30 : 0 },
      },
      yAxis: { type: "value", name: "Tickets", minInterval: 1 },
      series: [
        {
          name: "Vivants",
          type: "line",
          data: vivants,
          smooth: true,
          itemStyle: { color: "#0C419A" },
          lineStyle: { width: 2, color: "#0C419A" },
          symbol: "circle",
          symbolSize: 6,
        },
        {
          name: "Termines",
          type: "line",
          data: termines,
          smooth: true,
          itemStyle: { color: "#2E7D32" },
          lineStyle: { width: 2, color: "#2E7D32" },
          symbol: "circle",
          symbolSize: 6,
        },
      ],
    };
  }, [points]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');
  return <div ref={chartRef} style={{ height: "320px", width: "100%" }} />;
}

function TimelineView() {
  const timelineHook = useInvoke<TimelinePoint[]>();
  const [importIdA, setImportIdA] = useState<number | null>(null);
  const [importIdB, setImportIdB] = useState<number | null>(null);
  const [comparison, setComparison] = useState<ImportComparison | null>(null);
  const [comparing, setComparing] = useState(false);
  const [compareError, setCompareError] = useState<string | null>(null);

  useEffect(() => {
    timelineHook.execute("get_timeline_data");
  }, []);

  useEffect(() => {
    const pts = timelineHook.data;
    if (pts && pts.length >= 2) {
      setImportIdA(pts[pts.length - 2].importId);
      setImportIdB(pts[pts.length - 1].importId);
    }
  }, [timelineHook.data]);

  const handleCompare = async () => {
    if (importIdA === null || importIdB === null) return;
    setComparing(true);
    setCompareError(null);
    try {
      const result = await invoke<ImportComparison>("compare_imports", {
        importIdA,
        importIdB,
      });
      setComparison(result);
    } catch (err) {
      setCompareError(String(err));
    } finally {
      setComparing(false);
    }
  };

  const points = timelineHook.data ?? [];

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Longitudinal
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Evolution du stock entre imports successifs
        </p>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        {/* Error */}
        {timelineHook.error && (
          <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
            {timelineHook.error}
          </div>
        )}

        {/* Loading */}
        {timelineHook.loading && (
          <div className="flex items-center gap-3 py-8 text-slate-400">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary-500 border-t-transparent" />
            <span className="text-sm">Chargement...</span>
          </div>
        )}

        {/* Empty state */}
        {!timelineHook.loading && points.length < 2 && !timelineHook.error && (
          <div className="rounded-2xl bg-white py-16 text-center text-sm text-slate-400 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
            Au moins 2 imports necessaires pour la vue longitudinale
          </div>
        )}

        {points.length >= 2 && (
          <>
            <div className="animate-fade-slide-up rounded-2xl bg-white p-6 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
              <h2 className="mb-4 text-lg font-semibold font-[DM_Sans] text-slate-700">
                Evolution du stock
              </h2>
              <TimelineChart points={points} />
            </div>

            {/* Comparison */}
            <div className="animate-fade-slide-up animation-delay-150 rounded-2xl bg-white p-6 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] space-y-5">
              <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700">
                Comparaison d'imports
              </h2>

              <div className="flex flex-wrap items-end gap-4">
                <div>
                  <label className="block text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans] mb-1">
                    Import A
                  </label>
                  <select
                    value={importIdA ?? ""}
                    onChange={(e) => setImportIdA(Number(e.target.value))}
                    className="rounded-lg bg-white px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
                  >
                    {points.map((p) => (
                      <option key={p.importId} value={p.importId}>
                        {p.filename} — {formatDate(p.importDate)}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans] mb-1">
                    Import B
                  </label>
                  <select
                    value={importIdB ?? ""}
                    onChange={(e) => setImportIdB(Number(e.target.value))}
                    className="rounded-lg bg-white px-3 py-1.5 text-sm text-slate-800 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
                  >
                    {points.map((p) => (
                      <option key={p.importId} value={p.importId}>
                        {p.filename} — {formatDate(p.importDate)}
                      </option>
                    ))}
                  </select>
                </div>

                <button
                  onClick={handleCompare}
                  disabled={
                    comparing ||
                    importIdA === null ||
                    importIdB === null ||
                    importIdA === importIdB
                  }
                  className="rounded-xl bg-primary-500 px-4 py-2 text-sm font-medium text-white hover:bg-primary-600 disabled:opacity-50 transition-colors shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
                >
                  {comparing ? "Comparaison..." : "Comparer"}
                </button>
              </div>

              {compareError && (
                <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
                  {compareError}
                </div>
              )}

              {comparison && (
                <div className="space-y-5">
                  {/* Delta cards */}
                  <div className="grid grid-cols-2 gap-5 sm:grid-cols-4">
                    <DeltaCard label="Delta total" value={comparison.deltaTotal} />
                    <DeltaCard
                      label="Delta vivants"
                      value={comparison.deltaVivants}
                    />
                    <InfoCard
                      label="Nouveaux tickets"
                      value={comparison.nouveauxTickets.length}
                    />
                    <InfoCard
                      label="Disparus tickets"
                      value={comparison.disparusTickets.length}
                    />
                  </div>

                  {/* Delta par technicien */}
                  {comparison.deltaParTechnicien.length > 0 && (
                    <div>
                      <h3 className="mb-2 text-sm font-semibold font-[DM_Sans] text-slate-700">
                        Delta par technicien
                      </h3>
                      <div className="overflow-x-auto rounded-2xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
                        <table className="w-full text-sm">
                          <thead>
                            <tr className="border-b border-slate-100">
                              <th className="px-6 py-3.5 text-left text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
                                Technicien
                              </th>
                              <th className="px-6 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
                                Import A
                              </th>
                              <th className="px-6 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
                                Import B
                              </th>
                              <th className="px-6 py-3.5 text-right text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
                                Delta
                              </th>
                            </tr>
                          </thead>
                          <tbody>
                            {comparison.deltaParTechnicien.map((row) => (
                              <tr
                                key={row.technicien}
                                className="border-b border-slate-50 last:border-0 hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100"
                              >
                                <td className="px-6 py-3.5 text-slate-800">
                                  {row.technicien}
                                </td>
                                <td className="px-6 py-3.5 text-right text-slate-600 font-semibold font-[DM_Sans] tabular-nums">
                                  {row.countA}
                                </td>
                                <td className="px-6 py-3.5 text-right text-slate-600 font-semibold font-[DM_Sans] tabular-nums">
                                  {row.countB}
                                </td>
                                <td
                                  className={`px-6 py-3.5 text-right font-semibold font-[DM_Sans] tabular-nums ${
                                    row.delta > 0
                                      ? "text-danger-500"
                                      : row.delta < 0
                                        ? "text-success-500"
                                        : "text-slate-600"
                                  }`}
                                >
                                  {row.delta > 0 ? "+" : ""}
                                  {row.delta}
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export default TimelineView;
