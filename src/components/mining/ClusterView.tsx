import type { ClusterInfo } from "../../types/mining";

const MATERIAL_PALETTE = [
  "#1565C0",
  "#2E7D32",
  "#FF8F00",
  "#6A1B9A",
  "#00838F",
  "#C62828",
  "#4E342E",
  "#37474F",
];

function silhouetteBadge(score: number): { label: string; className: string } {
  if (score > 0.5) return { label: score.toFixed(2), className: "bg-success-50 text-success-500" };
  if (score > 0.25) return { label: score.toFixed(2), className: "bg-accent-50 text-accent-700" };
  return { label: score.toFixed(2), className: "bg-danger-50 text-danger-500" };
}

interface ClusterViewProps {
  clusters: ClusterInfo[];
  silhouetteScore: number;
  onClusterClick?: (cluster: ClusterInfo) => void;
}

export default function ClusterView({ clusters, silhouetteScore, onClusterClick }: ClusterViewProps) {
  const badge = silhouetteBadge(silhouetteScore);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <span className="text-sm text-slate-500">Score silhouette :</span>
        <span className={`rounded-lg px-3 py-1 text-sm font-semibold ${badge.className}`}>
          {badge.label}
        </span>
        <span className="text-xs text-slate-400">
          ({clusters.length} cluster{clusters.length !== 1 ? "s" : ""})
        </span>
      </div>

      <div className="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-3">
        {clusters.map((cluster) => {
          const color = MATERIAL_PALETTE[cluster.id % MATERIAL_PALETTE.length];
          return (
            <div
              key={cluster.id}
              onClick={() => onClusterClick?.(cluster)}
              className="relative overflow-hidden rounded-2xl bg-white p-5 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)] transition-shadow duration-200 cursor-pointer"
            >
              {/* Accent bar */}
              <div
                className="absolute top-0 inset-x-0 h-[3px] rounded-t-2xl"
                style={{ background: `linear-gradient(90deg, ${color}, ${color}88)` }}
              />

              <div className="mb-2 flex items-center justify-between gap-2">
                <span className="text-xs font-semibold uppercase tracking-wider font-[DM_Sans]" style={{ color }}>
                  Cluster {cluster.id + 1}
                </span>
                <span className="text-xs text-slate-400">
                  {cluster.ticketCount} ticket{cluster.ticketCount !== 1 ? "s" : ""}
                </span>
              </div>

              <p className="mb-3 text-sm font-medium text-slate-800 leading-snug">
                {cluster.label || cluster.topKeywords.slice(0, 3).join(" · ")}
              </p>

              <div className="mb-3 flex flex-wrap gap-1">
                {cluster.topKeywords.map((kw) => (
                  <span
                    key={kw}
                    className="rounded-lg px-2 py-0.5 text-xs font-medium"
                    style={{ backgroundColor: `${color}12`, color }}
                  >
                    {kw}
                  </span>
                ))}
              </div>

              {cluster.avgResolutionDays !== null && (
                <p className="text-xs text-slate-400">
                  Delai moy. resolution :{" "}
                  <strong className="text-slate-800 font-[DM_Sans]">
                    {cluster.avgResolutionDays.toFixed(1)} j
                  </strong>
                </p>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
