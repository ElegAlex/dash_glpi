import { useEffect, useMemo, useState } from 'react';
import { FolderTree, Layers, BarChart3 } from 'lucide-react';
import { useInvoke } from '../hooks/useInvoke';
import { useECharts } from '../hooks/useECharts';
import { Card } from '../components/shared/Card';
import { KpiCard } from '../components/shared/KpiCard';
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
        const path = p.treePathInfo?.map((x) => x.name).join(' > ') ?? p.name;
        return `<div style="font-size:13px;color:#1E293B">
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
            textStyle: { color: '#ffffff', fontSize: 11 },
          },
          emphasis: { itemStyle: { color: '#0a3783' } },
        },
        levels: [
          {
            itemStyle: { borderColor: '#E2E8F0', borderWidth: 2, gapWidth: 2 },
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
          fontSize: 10,
          color: '#ffffff',
        },
      },
    ],
  }), [tree]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');

  return <div ref={chartRef} className="h-[480px] w-full" />;
}

type Scope = 'vivants' | 'all';

function CategoriesPage() {
  const [scope, setScope] = useState<Scope>('vivants');
  const { data, loading, error, execute } = useInvoke<CategoryTree>();

  useEffect(() => {
    execute('get_categories_tree', { request: { scope } });
  }, [scope]); // eslint-disable-line react-hooks/exhaustive-deps

  const topCategory = useMemo(() => {
    if (!data) return null;
    return data.nodes.reduce<CategoryNode | null>(
      (max, node) => (!max || node.count > max.count ? node : max),
      null,
    );
  }, [data]);

  const isCategorie = data?.source === 'categorie';

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-4 border-b border-slate-200/30">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
              {isCategorie ? 'Categories ITIL' : 'Categories'}
            </h1>
            <p className="text-sm text-slate-400 mt-1">
              {isCategorie
                ? 'Arborescence des categories ITIL des tickets'
                : 'Arborescence des groupes de techniciens'}
            </p>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex rounded-lg overflow-hidden bg-slate-100">
              {([['vivants', 'En cours'], ['all', 'Tous les tickets']] as const).map(([val, label]) => (
                <button
                  key={val}
                  onClick={() => setScope(val)}
                  className={`px-3 py-1.5 text-xs font-medium font-[DM_Sans] transition-colors ${
                    scope === val
                      ? 'bg-[#0C419A] text-white'
                      : 'text-slate-500 hover:text-slate-700'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
            {data && (
              <span className="inline-flex items-center rounded-xl bg-primary-50 px-3 py-1 text-xs font-medium text-primary-500">
                Source : {isCategorie ? 'Categories ITIL' : 'Groupes de techniciens'}
              </span>
            )}
          </div>
        </div>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        {error && (
          <div className="rounded-2xl bg-danger-50 shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] px-4 py-3 text-sm text-danger-500">
            {error}
          </div>
        )}

        {loading && (
          <div className="flex items-center justify-center py-24 text-sm text-slate-400">
            Chargement de l'arborescence...
          </div>
        )}

        {data && !loading && (
          <>
            {/* KPI Cards */}
            <div className="animate-fade-slide-up grid grid-cols-3 gap-5">
              <KpiCard
                label="Total tickets"
                value={data.totalTickets}
                icon={<BarChart3 size={18} className="text-primary-500" />}
                accentColor="#0C419A"
              />
              <KpiCard
                label={isCategorie ? 'Categories' : 'Groupes'}
                value={data.nodes.length}
                icon={<FolderTree size={18} className="text-purple-600" />}
                accentColor="#6A1B9A"
              />
              <KpiCard
                label={isCategorie ? 'Top categorie' : 'Top groupe'}
                value={topCategory ? topCategory.name : '—'}
                icon={<Layers size={18} className="text-amber-600" />}
                accentColor="#FF8F00"
              />
            </div>

            {/* Treemap */}
            <div className="animate-fade-slide-up animation-delay-150">
              <Card>
                <div className="mb-3 flex items-center justify-between">
                  <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700">
                    Arborescence — {data.totalTickets.toLocaleString('fr-FR')} tickets
                  </h2>
                  <span className="text-xs text-slate-400">Cliquez sur un noeud pour zoomer</span>
                </div>
                <TreemapChart tree={data} />
              </Card>
            </div>

            {/* Detail table */}
            <div className="animate-fade-slide-up animation-delay-300">
              <div className="rounded-2xl bg-white overflow-hidden shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
                <div className="px-6 py-4">
                  <h2 className="text-lg font-semibold font-[DM_Sans] text-slate-700">
                    {isCategorie ? 'Detail par categorie' : 'Detail par groupe'}
                  </h2>
                </div>
                <div>
                  {data.nodes.map((node) => (
                    <div
                      key={node.fullPath}
                      className="flex items-center justify-between px-6 py-3.5 border-b border-slate-50 last:border-0 hover:bg-[rgba(12,65,154,0.04)] transition-colors duration-100"
                    >
                      <div>
                        <span className="font-medium text-sm text-slate-800">{node.name}</span>
                        {node.children.length > 0 && (
                          <span className="ml-2 text-xs text-slate-400">
                            ({node.children.length} {isCategorie ? 'sous-categories' : 'sous-groupes'})
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-4 text-sm">
                        <span className="text-slate-600 font-semibold font-[DM_Sans] tabular-nums">
                          {node.count.toLocaleString('fr-FR')} tickets
                        </span>
                        <span className="text-xs text-slate-400">
                          {node.percentage.toFixed(1)}%
                        </span>
                        <span className="text-xs text-slate-400">
                          Inc: {node.incidents} / Dem: {node.demandes}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export default CategoriesPage;
