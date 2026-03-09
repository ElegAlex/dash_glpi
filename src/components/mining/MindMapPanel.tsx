import { useState, useEffect, useMemo, useCallback } from "react";
import { X, ChevronDown } from "lucide-react";
import { useECharts } from "../../hooks/useECharts";
import { useInvoke } from "../../hooks/useInvoke";
import type { MindMapResult, MindMapNode, TicketRef } from "../../types/mining";

const PALETTE = [
  "#1565C0", "#2E7D32", "#FF8F00", "#6A1B9A",
  "#00838F", "#C62828", "#4E342E", "#37474F",
];

const DEPTH_OPACITY = ["", "CC", "99", "77", "55"];

interface TreeNode {
  name: string;
  value: number;
  chainKey: string;
  ticketCount: number;
  itemStyle: { color: string };
  label: { color: string; fontWeight?: string; fontSize?: number };
  children?: TreeNode[];
}

function buildTreeChildren(
  nodes: MindMapNode[],
  depth: number,
  colorIndex: number,
): TreeNode[] {
  return nodes.map((node, i) => {
    const ci = depth === 1 ? i : colorIndex;
    const color = PALETTE[ci % PALETTE.length] + (DEPTH_OPACITY[depth] ?? "55");
    const children = node.children.length > 0
      ? buildTreeChildren(node.children, depth + 1, ci)
      : undefined;
    return {
      name: `${node.word} (${node.ticketCount})`,
      value: node.weight,
      chainKey: "",
      ticketCount: node.ticketCount,
      itemStyle: { color },
      label: { color },
      children,
    };
  });
}

function assignChainKeys(nodes: TreeNode[], parentChain: string): void {
  for (const node of nodes) {
    const word = node.name.replace(/\s*\(\d+\)$/, "");
    node.chainKey = parentChain ? `${parentChain}|${word}` : word;
    if (node.children) {
      assignChainKeys(node.children, node.chainKey);
    }
  }
}

interface MindMapPanelProps {
  word: string;
  corpus?: string;
  includeResolved: boolean;
  onClose: () => void;
  onNavigate: (word: string) => void;
}

export default function MindMapPanel({
  word,
  corpus,
  includeResolved,
  onClose,
  onNavigate,
}: MindMapPanelProps) {
  const hook = useInvoke<MindMapResult>();
  const [drillDown, setDrillDown] = useState<{ title: string; tickets: TicketRef[] } | null>(null);

  useEffect(() => {
    hook.execute("get_cooccurrence_mindmap", {
      request: { word, corpus, includeResolved, maxBranches: 12, maxLeaves: 5, maxDepth: 5 },
    });
    setDrillDown(null);
  }, [word, includeResolved]); // eslint-disable-line react-hooks/exhaustive-deps

  const option = useMemo(() => {
    if (!hook.data) return {};

    const children = buildTreeChildren(hook.data.branches, 1, 0);
    assignChainKeys(children, hook.data.root);

    return {
      tooltip: {
        trigger: "item" as const,
        formatter: (raw: unknown) => {
          const params = raw as Record<string, unknown>;
          const data = params.data as { name: string; ticketCount?: number; value?: number } | undefined;
          if (!data) return "";
          const lines = [`<strong>${data.name}</strong>`];
          if (data.value != null) lines.push(`Poids co-occurrence: ${data.value}`);
          return lines.join("<br/>");
        },
      },
      series: [
        {
          type: "tree" as const,
          layout: "radial" as const,
          data: [
            {
              name: `${hook.data.root} (${hook.data.rootTicketCount})`,
              chainKey: hook.data.root,
              ticketCount: hook.data.rootTicketCount,
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
          symbolSize: (_value: unknown, params: unknown) => {
            const p = params as Record<string, unknown>;
            const data = p.data as { ticketCount?: number } | undefined;
            if (!data?.ticketCount) return 10;
            return Math.min(24, Math.max(8, 6 + Math.sqrt(data.ticketCount) * 1.5));
          },
          initialTreeDepth: 3,
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

  const handleClick = useCallback(
    (params: unknown) => {
      const p = params as Record<string, unknown>;
      const data = p.data as { name: string; chainKey?: string } | undefined;
      if (!data || !hook.data) return;

      const chainKey = data.chainKey;
      if (chainKey && hook.data.ticketMap[chainKey]) {
        const tickets = hook.data.ticketMap[chainKey];
        const chainWords = chainKey.split("|");
        const title = chainWords.length === 1
          ? `Tickets contenant "${chainWords[0]}"`
          : `Tickets : ${chainWords.map((w) => `"${w}"`).join(" + ")}`;
        setDrillDown({ title, tickets });
      }
    },
    [hook.data],
  );

  const onEvents = useMemo(
    () => ({
      click: handleClick,
      dblclick: (params: unknown) => {
        const p = params as Record<string, unknown>;
        const data = p.data as { name: string; children?: unknown[] } | undefined;
        if (!data) return;
        const w = data.name.replace(/\s*\(\d+\)$/, "");
        if (data.children && Array.isArray(data.children) && data.children.length > 0) {
          onNavigate(w);
        }
      },
    }),
    [handleClick, onNavigate],
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
              {" — "}clic = tickets, double-clic = recentrer
            </p>
          </div>
          <button
            onClick={onClose}
            className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-600 transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Chart area — shrinks when drill-down is open */}
        <div className={`relative transition-all duration-200 ${drillDown ? "h-[45%]" : "flex-1"} min-h-0`}>
          {hook.loading && (
            <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-slate-400 z-10">
              <div className="h-8 w-8 animate-spin rounded-full border-2 border-primary-500 border-t-transparent" />
              <span className="text-sm font-[Source_Sans_3]">Chargement de la carte mentale...</span>
            </div>
          )}

          {hook.error && (
            <div className="m-6 rounded-2xl bg-danger-50 px-4 py-3 text-sm text-danger-500">
              {hook.error}
            </div>
          )}

          <div
            ref={chartRef}
            style={{ width: "100%", height: "100%", visibility: hook.data && !hook.loading ? "visible" : "hidden" }}
          />
        </div>

        {/* Inline ticket list — appears below the chart */}
        {drillDown && (
          <div className="flex-1 min-h-0 flex flex-col border-t border-slate-200">
            {/* Drill-down header */}
            <div className="flex items-center justify-between px-5 py-3 bg-slate-50/80">
              <div>
                <h4 className="text-sm font-semibold font-[DM_Sans] text-slate-800">{drillDown.title}</h4>
                <p className="text-xs text-slate-400 mt-0.5">
                  {drillDown.tickets.length} ticket{drillDown.tickets.length !== 1 ? "s" : ""}
                </p>
              </div>
              <button
                onClick={() => setDrillDown(null)}
                className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-200 hover:text-slate-600 transition-colors"
              >
                <ChevronDown size={18} />
              </button>
            </div>

            {/* Ticket list */}
            <div className="flex-1 overflow-y-auto px-5 py-2 space-y-1.5">
              {drillDown.tickets.map((ticket) => (
                <div
                  key={ticket.id}
                  className="rounded-xl bg-white px-3 py-2 hover:bg-[rgba(12,65,154,0.04)] transition-colors shadow-[0_1px_3px_rgba(0,0,0,0.06)]"
                >
                  <span className="block text-xs font-semibold text-primary-500 font-[DM_Sans]">
                    #{ticket.id}
                  </span>
                  <span className="block text-sm text-slate-800 leading-snug mt-0.5">
                    {ticket.titre}
                  </span>
                </div>
              ))}
              {drillDown.tickets.length === 0 && (
                <p className="py-8 text-center text-sm text-slate-400">Aucun ticket</p>
              )}
            </div>
          </div>
        )}
      </div>
    </>
  );
}
