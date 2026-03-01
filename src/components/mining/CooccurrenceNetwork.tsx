import { useMemo } from "react";
import { useECharts } from "../../hooks/useECharts";
import type { CooccurrenceResult } from "../../types/mining";

const PALETTE = [
  "#1565C0", "#2E7D32", "#FF8F00", "#6A1B9A",
  "#00838F", "#C62828", "#4E342E", "#37474F",
];

interface CooccurrenceNetworkProps {
  data: CooccurrenceResult;
  height?: number;
  onNodeClick?: (word: string) => void;
  onEdgeClick?: (source: string, target: string) => void;
}

export default function CooccurrenceNetwork({
  data,
  height = 500,
  onNodeClick,
  onEdgeClick,
}: CooccurrenceNetworkProps) {
  const maxScore = Math.max(...data.nodes.map((n) => n.tfidfScore), 0.001);
  const minScore = Math.min(...data.nodes.map((n) => n.tfidfScore), 0);
  const maxWeight = Math.max(...data.edges.map((e) => e.weight), 1);

  const option = useMemo(
    () => ({
      tooltip: {
        trigger: "item" as const,
        formatter: (raw: unknown) => {
          const params = raw as Record<string, unknown>;
          if (params.dataType === "node") {
            const d = params.data as { name: string; tfidfScore: number; docFrequency: number };
            return (
              `<strong>${d.name}</strong><br/>` +
              `TF-IDF: ${d.tfidfScore.toFixed(3)}<br/>` +
              `${d.docFrequency} documents`
            );
          }
          if (params.dataType === "edge") {
            const d = params.data as { source: string; target: string; weight: number };
            return (
              `${d.source} — ${d.target}<br/>` +
              `Co-occurrences: ${d.weight}`
            );
          }
          return "";
        },
      },
      series: [
        {
          type: "graph" as const,
          layout: "force" as const,
          roam: true,
          draggable: true,
          force: {
            repulsion: 150,
            gravity: 0.06,
            edgeLength: [80, 220],
            friction: 0.6,
          },
          label: {
            show: true,
            position: "right" as const,
            fontSize: 11,
            fontFamily: "DM Sans, system-ui",
            color: "#334155",
          },
          emphasis: {
            focus: "adjacency" as const,
            label: { fontSize: 13, fontWeight: "bold" as const },
            lineStyle: { width: 3 },
          },
          data: data.nodes.map((node, i) => {
            const ratio =
              maxScore > minScore
                ? (node.tfidfScore - minScore) / (maxScore - minScore)
                : 0.5;
            return {
              name: node.id,
              symbolSize: 12 + ratio * 33,
              tfidfScore: node.tfidfScore,
              docFrequency: node.docFrequency,
              itemStyle: { color: PALETTE[i % PALETTE.length] },
            };
          }),
          links: data.edges.map((edge) => ({
            source: edge.source,
            target: edge.target,
            weight: edge.weight,
            lineStyle: {
              width: 1 + (edge.weight / maxWeight) * 4,
              color: "#CBD5E1",
              curveness: 0.1,
            },
          })),
        },
      ],
    }),
    [data, maxScore, minScore, maxWeight],
  );

  const onEvents = useMemo(
    () => ({
      click: (params: unknown) => {
        const p = params as Record<string, unknown>;
        if (p.dataType === "node" && onNodeClick) {
          const d = p.data as { name: string };
          onNodeClick(d.name);
        }
        if (p.dataType === "edge" && onEdgeClick) {
          const d = p.data as { source: string; target: string };
          onEdgeClick(d.source, d.target);
        }
      },
    }),
    [onNodeClick, onEdgeClick],
  );

  const { chartRef } = useECharts(option, onEvents, undefined);

  return <div ref={chartRef} style={{ width: "100%", height }} className="rounded-xl" />;
}
