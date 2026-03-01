import { AlertTriangle } from "lucide-react";
import type { AnomalyAlert } from "../../types/mining";

const SEVERITY_STYLES: Record<string, { badge: string; row: string }> = {
  high: {
    badge: "bg-danger-50 text-danger-500",
    row: "border-l-4 border-danger-500",
  },
  medium: {
    badge: "bg-accent-50 text-accent-700",
    row: "border-l-4 border-accent-500",
  },
};

function severityStyle(severity: string) {
  return SEVERITY_STYLES[severity.toLowerCase()] ?? {
    badge: "bg-slate-100 text-slate-500",
    row: "border-l-4 border-slate-300",
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
      <div className="flex items-center gap-2 text-sm text-slate-500">
        <AlertTriangle size={16} className="text-accent-500" />
        <span>
          <strong className="text-slate-800 font-[DM_Sans]">{anomalies.length}</strong> anomalie
          {anomalies.length !== 1 ? "s" : ""} detectee{anomalies.length !== 1 ? "s" : ""}
        </span>
      </div>

      {sorted.length === 0 && (
        <div className="rounded-2xl bg-white py-10 text-center text-sm text-slate-400 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
          Aucune anomalie detectee
        </div>
      )}

      <div className="space-y-2">
        {sorted.map((anomaly) => {
          const style = severityStyle(anomaly.severity);
          return (
            <div
              key={`${anomaly.ticketId}-${anomaly.anomalyType}`}
              className={`flex items-start gap-3 rounded-2xl bg-white px-4 py-3 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100 ${style.row}`}
            >
              <AlertTriangle
                size={16}
                className={
                  anomaly.severity.toLowerCase() === "high"
                    ? "mt-0.5 shrink-0 text-danger-500"
                    : "mt-0.5 shrink-0 text-accent-500"
                }
              />
              <div className="min-w-0 flex-1 space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className={`rounded-lg px-2 py-0.5 text-xs font-semibold uppercase ${style.badge}`}
                  >
                    {anomaly.severity}
                  </span>
                  <span className="rounded-md bg-slate-100 px-1.5 py-0.5 text-xs text-slate-500 font-[DM_Sans]">
                    {anomaly.anomalyType === "ticket_ancien" ? "Ancien" : anomaly.anomalyType === "ticket_inactif" ? "Inactif" : anomaly.anomalyType === "sans_suivi" ? "Sans suivi" : anomaly.anomalyType}
                  </span>
                  <span className="text-xs text-slate-400 font-[DM_Sans]">#{anomaly.ticketId}</span>
                  <span className="truncate text-sm font-medium text-slate-800">
                    {anomaly.titre}
                  </span>
                </div>
                <p className="text-xs text-slate-500">{anomaly.description}</p>
                <p className="text-xs text-slate-400">
                  Seuil : {anomaly.expectedRange}
                </p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
