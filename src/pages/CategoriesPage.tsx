import { useEffect, useMemo } from 'react';
import { useInvoke } from '../hooks/useInvoke';
import { useECharts } from '../hooks/useECharts';
import type { CategoryTree, CategoryNode } from '../types/kpi';
import type { EChartsCoreOption } from 'echarts/core';

function nodeToECharts(node: CategoryNode): object {
  return {
    name: node.name,
    value: node.count,
    children: node.children.length > 0 ? node.children.map(nodeToECharts) : undefined,
  };
}

function TreemapChart({ tree }: { tree: CategoryTree }) {
  const option = useMemo<EChartsCoreOption>(() => ({
    tooltip: {
      formatter: (params: unknown) => {
        const p = params as { name: string; value: number; treePathInfo: Array<{ name: string }> };
        const path = p.treePathInfo?.map((x) => x.name).join(' › ') ?? p.name;
        return `<div style="font-size:13px;color:#1a1f2e">
          <strong>${path}</strong><br/>
          ${p.value.toLocaleString('fr-FR')} ticket(s)
        </div>`;
      },
    },
    series: [
      {
        type: 'treemap',
        leafDepth: 1,
        nodeClick: 'zoomToNode',
        roam: false,
        animation: true,
        animationDurationUpdate: 300,
        breadcrumb: {
          show: true,
          left: 'center',
          top: 'bottom',
          height: 28,
          itemStyle: {
            color: '#0C419A',
            textStyle: { color: '#ffffff', fontSize: 13 },
          },
          emphasis: { itemStyle: { color: '#0a3783' } },
        },
        levels: [
          {
            itemStyle: { borderColor: '#e2e6ee', borderWidth: 2, gapWidth: 2 },
          },
          {
            colorSaturation: [0.35, 0.5],
            itemStyle: { borderColorSaturation: 0.6, gapWidth: 1, borderWidth: 1 },
          },
          {
            colorSaturation: [0.35, 0.5],
            itemStyle: { borderColorSaturation: 0.7, gapWidth: 1, borderWidth: 1 },
          },
        ],
        data: tree.nodes.map(nodeToECharts),
        label: {
          show: true,
          formatter: '{b}\n{c}',
          fontSize: 12,
          color: '#ffffff',
        },
      },
    ],
  }), [tree]);

  const { chartRef } = useECharts(option);

  return <div ref={chartRef} className="h-[480px] w-full" />;
}

function CategoriesPage() {
  const { data, loading, error, execute } = useInvoke<CategoryTree>();

  useEffect(() => {
    execute('get_categories_tree', {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold text-[#1a1f2e]">Catégories</h1>
        {data && (
          <span className="inline-flex items-center rounded-full bg-[#f0f4fa] border border-[#8baade] px-3 py-1 text-xs font-medium text-[#0C419A]">
            Source : {data.source === 'groupes' ? 'Groupes de techniciens' : 'Catégories ITIL'}
          </span>
        )}
      </div>

      {error && (
        <div className="rounded-md bg-[#fef2f2] border border-[#ce0500] px-4 py-3 text-sm text-[#af0400]">
          {error}
        </div>
      )}

      {loading && (
        <div className="flex items-center justify-center py-24 text-sm text-[#6e7891]">
          Chargement de l'arborescence…
        </div>
      )}

      {data && !loading && (
        <div className="rounded-lg border border-[#e2e6ee] bg-white p-4 shadow-[0_1px_3px_0_rgb(26_31_46/0.06)]">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-medium text-[#1a1f2e]">
              Arborescence — {data.totalTickets.toLocaleString('fr-FR')} tickets
            </h2>
            <span className="text-xs text-[#6e7891]">Cliquez sur un nœud pour zoomer</span>
          </div>
          <TreemapChart tree={data} />
        </div>
      )}

      {data && !loading && (
        <div className="rounded-lg border border-[#e2e6ee] bg-white shadow-[0_1px_3px_0_rgb(26_31_46/0.06)]">
          <div className="px-4 py-3 border-b border-[#e2e6ee]">
            <h2 className="text-base font-medium text-[#1a1f2e]">Détail par groupe</h2>
          </div>
          <div className="divide-y divide-[#e2e6ee]">
            {data.nodes.map((node) => (
              <div key={node.fullPath} className="flex items-center justify-between px-4 py-3 hover:bg-[#f8f9fb]">
                <div>
                  <span className="font-medium text-sm text-[#1a1f2e]">{node.name}</span>
                  {node.children.length > 0 && (
                    <span className="ml-2 text-xs text-[#6e7891]">({node.children.length} sous-groupes)</span>
                  )}
                </div>
                <div className="flex items-center gap-4 text-sm">
                  <span className="text-[#525d73]">
                    {node.count.toLocaleString('fr-FR')} tickets
                  </span>
                  <span className="text-xs text-[#6e7891]">
                    {node.percentage.toFixed(1)}%
                  </span>
                  <span className="text-xs text-[#6e7891]">
                    Inc: {node.incidents} / Dem: {node.demandes}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export default CategoriesPage;
