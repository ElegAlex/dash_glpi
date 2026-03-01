import type { ClusterInfo } from "../../types/mining";

const CPAM_PALETTE = [
  "#0C419A",
  "#E69F00",
  "#009E73",
  "#D55E00",
  "#56B4E9",
  "#CC79A7",
  "#0072B2",
  "#228833",
];

function silhouetteBadge(score: number): { label: string; className: string } {
  if (score > 0.5) return { label: score.toFixed(2), className: "bg-green-100 text-green-700" };
  if (score > 0.25) return { label: score.toFixed(2), className: "bg-yellow-100 text-yellow-700" };
  return { label: score.toFixed(2), className: "bg-red-100 text-red-700" };
}

interface ClusterViewProps {
  clusters: ClusterInfo[];
  silhouetteScore: number;
}

export default function ClusterView({ clusters, silhouetteScore }: ClusterViewProps) {
  const badge = silhouetteBadge(silhouetteScore);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <span className="text-sm text-[#6e7891]">Score silhouette :</span>
        <span className={`rounded-full px-3 py-1 text-sm font-semibold ${badge.className}`}>
          {badge.label}
        </span>
        <span className="text-xs text-[#6e7891]">
          ({clusters.length} cluster{clusters.length !== 1 ? "s" : ""})
        </span>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
        {clusters.map((cluster) => {
          const color = CPAM_PALETTE[cluster.id % CPAM_PALETTE.length];
          return (
            <div
              key={cluster.id}
              className="rounded-xl border border-[#e2e6ee] bg-white p-4 shadow-sm"
              style={{ borderLeftColor: color, borderLeftWidth: 4 }}
            >
              <div className="mb-2 flex items-center justify-between gap-2">
                <span className="text-xs font-semibold uppercase tracking-wide" style={{ color }}>
                  Cluster {cluster.id + 1}
                </span>
                <span className="text-xs text-[#6e7891]">
                  {cluster.ticketCount} ticket{cluster.ticketCount !== 1 ? "s" : ""}
                </span>
              </div>

              <p className="mb-3 text-sm font-medium text-[#1a1f2e] leading-snug">
                {cluster.label || cluster.topKeywords.slice(0, 3).join(" · ")}
              </p>

              <div className="mb-3 flex flex-wrap gap-1">
                {cluster.topKeywords.map((kw) => (
                  <span
                    key={kw}
                    className="rounded-full px-2 py-0.5 text-xs font-medium"
                    style={{ backgroundColor: `${color}18`, color }}
                  >
                    {kw}
                  </span>
                ))}
              </div>

              {cluster.avgResolutionDays !== null && (
                <p className="text-xs text-[#6e7891]">
                  Délai moy. résolution :{" "}
                  <strong className="text-[#1a1f2e]">
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
