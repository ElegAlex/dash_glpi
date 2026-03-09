import { useEffect, useMemo } from "react";
import { X } from "lucide-react";
import { useECharts } from "../../hooks/useECharts";
import { useInvoke } from "../../hooks/useInvoke";
import type { MindMapResult } from "../../types/mining";

const PALETTE = [
  "#1565C0", "#2E7D32", "#FF8F00", "#6A1B9A",
  "#00838F", "#C62828", "#4E342E", "#37474F",
];

interface MindMapPanelProps {
  word: string;
  includeResolved: boolean;
  onClose: () => void;
  onNavigate: (word: string) => void;
}

export default function MindMapPanel({
  word,
  includeResolved,
  onClose,
  onNavigate,
}: MindMapPanelProps) {
  const hook = useInvoke<MindMapResult>();

  useEffect(() => {
    hook.execute("get_cooccurrence_mindmap", {
      request: { word, includeResolved, maxBranches: 15, maxLeaves: 5 },
    });
  }, [word, includeResolved]); // eslint-disable-line react-hooks/exhaustive-deps

  const option = useMemo(() => {
    if (!hook.data) return {};

    const children = hook.data.branches.map((branch, i) => {
      const color = PALETTE[i % PALETTE.length];
      return {
        name: branch.word,
        value: branch.weight,
        itemStyle: { color },
        label: { color },
        children: branch.children.map((leaf) => ({
          name: leaf.word,
          value: leaf.weight,
          itemStyle: { color: color + "99" },
          label: { color: color + "99" },
        })),
      };
    });

    return {
      tooltip: {
        trigger: "item" as const,
        formatter: (raw: unknown) => {
          const params = raw as Record<string, unknown>;
          const data = params.data as { name: string; value?: number } | undefined;
          if (!data) return "";
          if (data.value != null) {
            return `<strong>${data.name}</strong><br/>Poids: ${data.value}`;
          }
          return `<strong>${data.name}</strong>`;
        },
      },
      series: [
        {
          type: "tree" as const,
          layout: "radial" as const,
          data: [
            {
              name: hook.data.root,
              itemStyle: { color: "#0C419A" },
              label: {
                color: "#0C419A",
                fontWeight: "bold" as const,
                fontSize: 14,
              },
              children,
            },
          ],
          symbol: "circle",
          symbolSize: 14,
          initialTreeDepth: 2,
          roam: true,
          label: {
            fontSize: 11,
            fontFamily: "DM Sans, system-ui",
          },
          lineStyle: {
            color: "#CBD5E1",
            width: 1.5,
            curveness: 0.5,
          },
          emphasis: {
            focus: "ancestor" as const,
          },
        },
      ],
    };
  }, [hook.data]);

  const onEvents = useMemo(
    () => ({
      click: (params: unknown) => {
        const p = params as Record<string, unknown>;
        const data = p.data as { name: string; children?: unknown[] } | undefined;
        if (!data) return;
        // Only navigate on degree-1 nodes (those with children)
        if (data.children && Array.isArray(data.children) && data.children.length > 0) {
          onNavigate(data.name);
        }
      },
    }),
    [onNavigate],
  );

  const { chartRef } = useECharts(option, onEvents, undefined);

  return (
    <>
      {/* Overlay */}
      <div className="fixed inset-0 z-40 bg-black/20" onClick={onClose} />

      {/* Drawer */}
      <div className="fixed inset-y-0 right-0 z-50 w-[55%] min-w-[500px] max-w-[900px] bg-white shadow-[0_15px_30px_rgba(0,0,0,0.12),0_5px_15px_rgba(0,0,0,0.08)] rounded-l-2xl flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-200/50">
          <div>
            <h3 className="text-base font-semibold font-[DM_Sans] text-slate-800">
              Carte mentale
            </h3>
            <p className="text-xs text-slate-400 mt-0.5 font-[Source_Sans_3]">
              Centre : <span className="font-semibold text-primary-500">{word}</span>
              {" — "}cliquez sur un noeud pour recentrer
            </p>
          </div>
          <button
            onClick={onClose}
            className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-600 transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 min-h-0">
          {hook.loading && (
            <div className="flex flex-col items-center justify-center h-full gap-3 text-slate-400">
              <div className="h-8 w-8 animate-spin rounded-full border-2 border-primary-500 border-t-transparent" />
              <span className="text-sm font-[Source_Sans_3]">Chargement de la carte mentale...</span>
            </div>
          )}

          {hook.error && (
            <div className="m-6 rounded-2xl bg-danger-50 px-4 py-3 text-sm text-danger-500">
              {hook.error}
            </div>
          )}

          {!hook.loading && !hook.error && hook.data && (
            <div ref={chartRef} style={{ width: "100%", height: "100%" }} />
          )}

          {!hook.loading && !hook.error && !hook.data && (
            <div className="flex items-center justify-center h-full text-sm text-slate-400">
              Aucune donnee
            </div>
          )}
        </div>
      </div>
    </>
  );
}
