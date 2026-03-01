import { AlertTriangle } from "lucide-react";
import type { AnomalyAlert } from "../../types/mining";

const SEVERITY_STYLES: Record<string, { badge: string; row: string }> = {
  high: {
    badge: "bg-red-50 text-red-600",
    row: "border-l-4 border-red-400",
  },
  medium: {
    badge: "bg-yellow-50 text-yellow-700",
    row: "border-l-4 border-yellow-400",
  },
};

function severityStyle(severity: string) {
  return SEVERITY_STYLES[severity.toLowerCase()] ?? {
    badge: "bg-gray-100 text-gray-600",
    row: "border-l-4 border-gray-300",
  };
}

interface AnomalyListProps {
  anomalies: AnomalyAlert[];
}

export default function AnomalyList({ anomalies }: AnomalyListProps) {
  const sorted = [...anomalies].sort((a, b) => {
    const order = ["high", "medium"];
    return order.indexOf(a.severity.toLowerCase()) - order.indexOf(b.severity.toLowerCase());
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 text-sm text-[#6e7891]">
        <AlertTriangle size={16} className="text-yellow-500" />
        <span>
          <strong className="text-[#1a1f2e]">{anomalies.length}</strong> anomalie
          {anomalies.length !== 1 ? "s" : ""} détectée{anomalies.length !== 1 ? "s" : ""}
        </span>
      </div>

      {sorted.length === 0 && (
        <div className="rounded-xl border border-dashed border-[#cdd3df] bg-white py-10 text-center text-sm text-[#6e7891]">
          Aucune anomalie détectée
        </div>
      )}

      <div className="space-y-2">
        {sorted.map((anomaly) => {
          const style = severityStyle(anomaly.severity);
          return (
            <div
              key={`${anomaly.ticketId}-${anomaly.anomalyType}`}
              className={`flex items-start gap-3 rounded-lg bg-white px-4 py-3 shadow-sm hover:bg-[#f8f9fb] transition-colors ${style.row}`}
            >
              <AlertTriangle
                size={16}
                className={
                  anomaly.severity.toLowerCase() === "high"
                    ? "mt-0.5 shrink-0 text-red-500"
                    : "mt-0.5 shrink-0 text-yellow-500"
                }
              />
              <div className="min-w-0 flex-1 space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className={`rounded-full px-2 py-0.5 text-xs font-semibold uppercase ${style.badge}`}
                  >
                    {anomaly.severity}
                  </span>
                  <span className="text-xs text-[#6e7891]">#{anomaly.ticketId}</span>
                  <span className="truncate text-sm font-medium text-[#1a1f2e]">
                    {anomaly.titre}
                  </span>
                </div>
                <p className="text-xs text-[#6e7891]">
                  <strong className="text-[#1a1f2e]">Z-score :</strong>{" "}
                  {anomaly.metricValue.toFixed(2)} — {anomaly.description}
                </p>
                <p className="text-xs text-[#6e7891]">
                  Plage attendue : {anomaly.expectedRange}
                </p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
