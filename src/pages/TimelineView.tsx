import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GitCompareArrows } from "lucide-react";
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
      className={`rounded-lg border-l-4 px-4 py-3 ${
        positive
          ? "border-[#18753c] bg-[#f0faf4]"
          : "border-[#ce0500] bg-[#fef2f2]"
      }`}
    >
      <span className="block text-xs text-[#6e7891]">{label}</span>
      <span
        className={`text-xl font-bold ${
          positive ? "text-[#18753c]" : "text-[#ce0500]"
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
    <div className="rounded-lg border-l-4 border-[#0C419A] bg-[#f8f9fb] px-4 py-3">
      <span className="block text-xs text-[#6e7891]">{label}</span>
      <span className="text-xl font-bold text-[#1a1f2e]">{value}</span>
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
            `Terminés : ${pt.terminesCount}`,
            `Total : ${pt.totalRows}`,
          ].join("<br/>");
        },
      },
      legend: { data: ["Vivants", "Terminés"], top: 8 },
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
          name: "Terminés",
          type: "line",
          data: termines,
          smooth: true,
          itemStyle: { color: "#009E73" },
          lineStyle: { width: 2, color: "#009E73" },
          symbol: "circle",
          symbolSize: 6,
        },
      ],
    };
  }, [points]);

  const { chartRef } = useECharts(option);
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
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <GitCompareArrows size={22} className="text-[#0C419A]" />
        <h1 className="text-2xl font-semibold text-[#1a1f2e]">Longitudinal</h1>
      </div>

      {/* Error */}
      {timelineHook.error && (
        <div className="rounded-md border border-[#ce0500] bg-[#fef2f2] px-4 py-3 text-sm text-[#af0400]">
          {timelineHook.error}
        </div>
      )}

      {/* Loading */}
      {timelineHook.loading && (
        <div className="flex items-center gap-3 py-8 text-[#6e7891]">
          <div className="h-6 w-6 animate-spin rounded-full border-2 border-[#0C419A] border-t-transparent" />
          <span className="text-sm">Chargement…</span>
        </div>
      )}

      {/* Section A — Evolution chart */}
      {!timelineHook.loading && points.length < 2 && !timelineHook.error && (
        <div className="rounded-xl border border-dashed border-[#cdd3df] bg-white py-16 text-center text-sm text-[#6e7891]">
          Au moins 2 imports nécessaires pour la vue longitudinale
        </div>
      )}

      {points.length >= 2 && (
        <>
          <div className="rounded-xl border border-[#e2e6ee] bg-white p-4 shadow-sm">
            <h2 className="mb-4 text-base font-semibold text-[#1a1f2e]">
              Évolution du stock
            </h2>
            <TimelineChart points={points} />
          </div>

          {/* Section B — Comparison */}
          <div className="rounded-xl border border-[#e2e6ee] bg-white p-5 shadow-sm space-y-5">
            <h2 className="text-base font-semibold text-[#1a1f2e]">
              Comparaison d'imports
            </h2>

            <div className="flex flex-wrap items-end gap-4">
              <div>
                <label className="block text-xs font-medium text-[#525d73] mb-1">
                  Import A
                </label>
                <select
                  value={importIdA ?? ""}
                  onChange={(e) => setImportIdA(Number(e.target.value))}
                  className="border border-[#e2e6ee] rounded px-3 py-1.5 text-sm text-[#1a1f2e] focus:outline-none focus:border-[#0C419A]"
                >
                  {points.map((p) => (
                    <option key={p.importId} value={p.importId}>
                      {p.filename} — {formatDate(p.importDate)}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-[#525d73] mb-1">
                  Import B
                </label>
                <select
                  value={importIdB ?? ""}
                  onChange={(e) => setImportIdB(Number(e.target.value))}
                  className="border border-[#e2e6ee] rounded px-3 py-1.5 text-sm text-[#1a1f2e] focus:outline-none focus:border-[#0C419A]"
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
                className="rounded-lg bg-[#0C419A] px-4 py-2 text-sm font-medium text-white hover:bg-[#0a3783] disabled:opacity-50 transition-colors"
              >
                {comparing ? "Comparaison…" : "Comparer"}
              </button>
            </div>

            {compareError && (
              <div className="rounded-md border border-[#ce0500] bg-[#fef2f2] px-4 py-3 text-sm text-[#af0400]">
                {compareError}
              </div>
            )}

            {comparison && (
              <div className="space-y-5">
                {/* Delta cards */}
                <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
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
                    <h3 className="mb-2 text-sm font-semibold text-[#1a1f2e]">
                      Delta par technicien
                    </h3>
                    <div className="overflow-x-auto rounded-lg border border-[#e2e6ee]">
                      <table className="w-full text-sm">
                        <thead className="bg-[#f8f9fb]">
                          <tr>
                            <th className="px-4 py-2.5 text-left font-medium text-[#525d73]">
                              Technicien
                            </th>
                            <th className="px-4 py-2.5 text-right font-medium text-[#525d73]">
                              Import A
                            </th>
                            <th className="px-4 py-2.5 text-right font-medium text-[#525d73]">
                              Import B
                            </th>
                            <th className="px-4 py-2.5 text-right font-medium text-[#525d73]">
                              Delta
                            </th>
                          </tr>
                        </thead>
                        <tbody>
                          {comparison.deltaParTechnicien.map((row) => (
                            <tr
                              key={row.technicien}
                              className="border-t border-[#e2e6ee] hover:bg-[#f8f9fb]"
                            >
                              <td className="px-4 py-2 text-[#1a1f2e]">
                                {row.technicien}
                              </td>
                              <td className="px-4 py-2 text-right text-[#525d73]">
                                {row.countA}
                              </td>
                              <td className="px-4 py-2 text-right text-[#525d73]">
                                {row.countB}
                              </td>
                              <td
                                className={`px-4 py-2 text-right font-medium ${
                                  row.delta > 0
                                    ? "text-[#ce0500]"
                                    : row.delta < 0
                                      ? "text-[#18753c]"
                                      : "text-[#525d73]"
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
  );
}

export default TimelineView;
