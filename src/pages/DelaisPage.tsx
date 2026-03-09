import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { subDays } from 'date-fns';
import { invoke } from '@tauri-apps/api/core';
import * as echarts from 'echarts/core';
import { Clock, Zap, Timer, Target, X, Filter } from 'lucide-react';
import { CompactFilterBar } from '../components/shared/CompactFilterBar';
import { KpiCard } from '../components/shared/KpiCard';
import { Card } from '../components/shared/Card';
import { CategorieDelaisTable } from '../components/delais/CategorieDelaisTable';
import { useInvoke } from '../hooks/useInvoke';
import { useECharts } from '../hooks/useECharts';
import type { DelaisKpi, TrancheDelai } from '../types/delais';
import type { CategorieDelais } from '../types/delais';
import type { ImportHistory } from '../types/config';
import { type DateRange, type Granularity } from '../components/shared/DateRangePicker';
import type { EChartsCoreOption } from 'echarts/core';
import '../lib/echarts-theme';

function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

const DISTRIBUTION_COLORS = ['#2E7D32', '#1565C0', '#FF8F00', '#6A1B9A', '#C62828'];

function TrendChart({ data }: { data: DelaisKpi }) {
  const option = useMemo<EChartsCoreOption>(() => {
    const labels = data.trend.map((t) => t.periodLabel);
    const pct24 = data.trend.map((t) => t.pct24h);
    const pct48 = data.trend.map((t) => t.pct48h);

    return {
      tooltip: {
        trigger: 'axis' as const,
        formatter: (params: Array<{ seriesName: string; value: number; marker: string; dataIndex: number }>) => {
          const idx = params[0]?.dataIndex ?? 0;
          const period = labels[idx] ?? '';
          const total = data.trend[idx]?.totalResolus ?? 0;
          let html = `<div style="font-weight:600;margin-bottom:4px">${period}</div>`;
          for (const p of params) {
            html += `<div>${p.marker} ${p.seriesName}: <b>${p.value}%</b></div>`;
          }
          html += `<div style="margin-top:4px;color:#64748B;font-size:12px">${total.toLocaleString('fr-FR')} ticket(s) resolus</div>`;
          return html;
        },
      },
      legend: {
        data: ['Taux 24h', 'Taux 48h'],
        top: 8,
      },
      grid: { top: 60, bottom: 30, left: 50, right: 30, containLabel: true },
      xAxis: {
        type: 'category' as const,
        data: labels,
        axisLabel: { rotate: labels.length > 6 ? 90 : 0 },
      },
      yAxis: {
        type: 'value' as const,
        name: '%',
        max: 100,
        minInterval: 10,
      },
      series: [
        {
          name: 'Taux 24h',
          type: 'line' as const,
          data: pct24,
          smooth: true,
          symbol: 'circle',
          symbolSize: 7,
          lineStyle: { width: 3, color: '#2E7D32' },
          itemStyle: { color: '#2E7D32', borderColor: '#FFF', borderWidth: 2 },
          markLine: {
            silent: true,
            symbol: 'none',
            data: [
              {
                yAxis: 80,
                lineStyle: { type: 'dotted' as const, color: '#C62828', width: 2 },
                label: {
                  formatter: 'Objectif: 80%',
                  position: 'insideEndTop' as const,
                  color: '#C62828',
                  fontSize: 11,
                  fontWeight: 600,
                },
              },
            ],
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(46,125,50,0.15)' },
              { offset: 1, color: 'rgba(46,125,50,0.02)' },
            ]),
          },
        },
        {
          name: 'Taux 48h',
          type: 'line' as const,
          data: pct48,
          smooth: true,
          symbol: 'circle',
          symbolSize: 7,
          lineStyle: { width: 3, color: '#1565C0' },
          itemStyle: { color: '#1565C0', borderColor: '#FFF', borderWidth: 2 },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(21,101,192,0.15)' },
              { offset: 1, color: 'rgba(21,101,192,0.02)' },
            ]),
          },
        },
      ],
    };
  }, [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');
  return <div ref={chartRef} style={{ height: '360px', width: '100%' }} />;
}

function DistributionChart({ tranches }: { tranches: TrancheDelai[] }) {
  const option = useMemo<EChartsCoreOption>(() => {
    const labels = tranches.map((t) => t.label);
    const values = tranches.map((t) => t.count);

    return {
      tooltip: {
        trigger: 'axis' as const,
        axisPointer: { type: 'shadow' as const },
        formatter: (params: unknown) => {
          const p = (params as Array<{ name: string; value: number }>)[0];
          const tranche = tranches.find((t) => t.label === p.name);
          return `<div style="font-size:13px;color:#1E293B">
            <strong>${p.name}</strong><br/>
            ${p.value.toLocaleString('fr-FR')} ticket(s)
            ${tranche ? ` (${tranche.pourcentage}%)` : ''}
          </div>`;
        },
      },
      grid: { left: 100, right: 60, top: 10, bottom: 30 },
      xAxis: { type: 'value' as const },
      yAxis: {
        type: 'category' as const,
        data: labels,
        inverse: true,
        axisLabel: { fontFamily: 'DM Sans' },
      },
      series: [
        {
          type: 'bar' as const,
          data: values.map((val, idx) => ({
            value: val,
            itemStyle: {
              color: DISTRIBUTION_COLORS[idx % DISTRIBUTION_COLORS.length],
              borderRadius: [0, 6, 6, 0],
            },
          })),
          barWidth: '55%',
          label: {
            show: true,
            position: 'right' as const,
            fontSize: 11,
            color: '#64748B',
            formatter: (params: { value: number; dataIndex: number }) => {
              const t = tranches[params.dataIndex];
              return `${params.value.toLocaleString('fr-FR')} (${t.pourcentage}%)`;
            },
          },
        },
      ],
    };
  }, [tranches]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');
  return <div ref={chartRef} className="h-[240px] w-full" />;
}

const CUMUL_THRESHOLDS = [
  { label: '24h', index: 0, color: '#2E7D32' },
  { label: '48h', index: 1, color: '#1565C0' },
  { label: '7j', index: 2, color: '#FF8F00' },
  { label: '30j', index: 3, color: '#6A1B9A' },
];

function CumulativeBar({ tranches }: { tranches: TrancheDelai[] }) {
  const cumulatives = useMemo(() => {
    let cumul = 0;
    return CUMUL_THRESHOLDS.map((t) => {
      cumul += tranches[t.index]?.pourcentage ?? 0;
      const cumulCount = tranches
        .slice(0, t.index + 1)
        .reduce((s, tr) => s + tr.count, 0);
      return { ...t, pct: Math.round(cumul * 10) / 10, count: cumulCount };
    });
  }, [tranches]);

  return (
    <div className="space-y-4">
      {/* Progress bar */}
      <div className="relative h-6 bg-slate-100 rounded-full overflow-hidden">
        {[...cumulatives].reverse().map((c) => (
          <div
            key={c.label}
            className="absolute inset-y-0 left-0 rounded-full transition-all duration-500"
            style={{ width: `${c.pct}%`, backgroundColor: c.color, opacity: 0.75 }}
          />
        ))}
      </div>
      {/* Milestone cards */}
      <div className="grid grid-cols-4 gap-3">
        {cumulatives.map((c) => (
          <div
            key={c.label}
            className="rounded-xl bg-white shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-3 text-center"
          >
            <div className="flex items-center justify-center gap-1.5 mb-1">
              <div className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: c.color }} />
              <span className="text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
                {`\u2264 ${c.label}`}
              </span>
            </div>
            <p className="text-2xl font-bold font-[DM_Sans] tracking-tight" style={{ color: c.color }}>
              {c.pct}%
            </p>
            <p className="text-xs text-slate-400 mt-0.5">
              {c.count.toLocaleString('fr-FR')} ticket(s)
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}

function DelaisPage() {
  const today = new Date();
  const [range, setRange] = useState<DateRange>({ from: subDays(today, 29), to: today });
  const [granularity, setGranularity] = useState<Granularity>('month');
  const { data, loading, error, execute } = useInvoke<DelaisKpi>();
  const initialized = useRef(false);
  const [activeTab, setActiveTab] = useState<'synthese' | 'par-categorie'>('synthese');
  const [catNiveau1, setCatNiveau1] = useState<string | null>(null);
  const [catNiveau2, setCatNiveau2] = useState<string | null>(null);
  const [categorie, setCategorie] = useState<string | null>(null);
  const [catOptions1, setCatOptions1] = useState<string[]>([]);
  const [catOptions2, setCatOptions2] = useState<string[]>([]);
  const [catOptions3, setCatOptions3] = useState<string[]>([]);
  const [catTableData, setCatTableData] = useState<CategorieDelais[]>([]);
  const [catTableLoading, setCatTableLoading] = useState(false);

  const load = useCallback(
    (r: DateRange, g: Granularity, cn1?: string | null, cn2?: string | null, cat?: string | null) => {
      execute('get_delais_kpi', {
        request: {
          dateFrom: formatDate(r.from),
          dateTo: formatDate(r.to),
          granularity: g,
          categorieNiveau1: cn1 || undefined,
          categorieNiveau2: cn2 || undefined,
          categorie: cat || undefined,
        },
      });
    },
    [execute],
  );

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;
    invoke<ImportHistory[]>('get_import_history').then((history) => {
      const active = history.find((h) => h.isActive);
      if (active?.dateRangeFrom && active?.dateRangeTo) {
        const from = new Date(active.dateRangeFrom);
        const to = new Date(active.dateRangeTo);
        setRange({ from, to });
        const days = (to.getTime() - from.getTime()) / 86400000;
        const g: Granularity = days > 365 ? 'quarter' : days > 60 ? 'month' : days > 14 ? 'week' : 'day';
        setGranularity(g);
        load({ from, to }, g, null, null, null);
      } else {
        load(range, granularity, null, null, null);
      }
    }).catch(() => {
      load(range, granularity, null, null, null);
    });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const loadCatOptions = useCallback(async (
    column: string,
    parentColumn?: string,
    parentValue?: string,
  ) => {
    return invoke<string[]>('get_distinct_categories_for_delais', {
      request: {
        dateFrom: formatDate(range.from),
        dateTo: formatDate(range.to),
        column,
        parentColumn,
        parentValue,
      },
    });
  }, [range]);

  useEffect(() => {
    if (!data) return;
    loadCatOptions('categorie_niveau1').then(setCatOptions1).catch(() => setCatOptions1([]));
  }, [data, loadCatOptions]);

  useEffect(() => {
    if (!catNiveau1) { setCatOptions2([]); return; }
    loadCatOptions('categorie_niveau2', 'categorie_niveau1', catNiveau1)
      .then(setCatOptions2).catch(() => setCatOptions2([]));
  }, [catNiveau1, loadCatOptions]);

  useEffect(() => {
    if (!catNiveau2) { setCatOptions3([]); return; }
    loadCatOptions('categorie', 'categorie_niveau2', catNiveau2)
      .then(setCatOptions3).catch(() => setCatOptions3([]));
  }, [catNiveau2, loadCatOptions]);

  const loadCatTable = useCallback(async () => {
    setCatTableLoading(true);
    try {
      const result = await invoke<CategorieDelais[]>('get_delais_par_categorie', {
        request: {
          dateFrom: formatDate(range.from),
          dateTo: formatDate(range.to),
          categorieNiveau1: catNiveau1 || undefined,
          categorieNiveau2: catNiveau2 || undefined,
          categorie: categorie || undefined,
        },
      });
      setCatTableData(result);
    } catch {
      setCatTableData([]);
    } finally {
      setCatTableLoading(false);
    }
  }, [range, catNiveau1, catNiveau2, categorie]);

  useEffect(() => {
    if (activeTab === 'par-categorie') {
      loadCatTable();
    }
  }, [activeTab, loadCatTable]);

  const handleCatNiveau1Change = (val: string) => {
    const v = val || null;
    setCatNiveau1(v);
    setCatNiveau2(null);
    setCategorie(null);
    load(range, granularity, v, null, null);
  };

  const handleCatNiveau2Change = (val: string) => {
    const v = val || null;
    setCatNiveau2(v);
    setCategorie(null);
    load(range, granularity, catNiveau1, v, null);
  };

  const handleCategorieChange = (val: string) => {
    const v = val || null;
    setCategorie(v);
    load(range, granularity, catNiveau1, catNiveau2, v);
  };

  const resetCatFilter = () => {
    setCatNiveau1(null);
    setCatNiveau2(null);
    setCategorie(null);
    load(range, granularity, null, null, null);
  };

  const handleRangeChange = (r: DateRange, autoGran: Granularity) => {
    setRange(r);
    setGranularity(autoGran);
    load(r, autoGran, catNiveau1, catNiveau2, categorie);
  };

  const handleGranularityChange = (g: Granularity) => {
    setGranularity(g);
    load(range, g, catNiveau1, catNiveau2, categorie);
  };

  return (
    <div>
      <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm px-8 pt-6 pb-3 border-b border-slate-200/30">
        <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
          Delais de prise en charge
        </h1>
        <p className="text-sm text-slate-400 mt-0.5">
          Taux de traitement en 24h et 48h par periode
        </p>
        <div className="mt-3">
          <CompactFilterBar
            range={range}
            granularity={granularity}
            onRangeChange={handleRangeChange}
            onGranularityChange={handleGranularityChange}
          />
        </div>

        {/* Category filter */}
        <div className="flex items-center gap-3 flex-wrap mt-2">
          {catOptions1.length > 0 && (
            <>
              <div className="h-5 border-l border-slate-200/60" />
              <Filter size={14} className="text-slate-400" />
              <select
                value={catNiveau1 || ''}
                onChange={(e) => handleCatNiveau1Change(e.target.value)}
                className="text-xs font-[DM_Sans] bg-slate-100 rounded-lg px-2.5 py-1.5 text-slate-700 border-none outline-none"
              >
                <option value="">Toutes categories</option>
                {catOptions1.map((c) => <option key={c} value={c}>{c}</option>)}
              </select>

              {catNiveau1 && catOptions2.length > 0 && (
                <select
                  value={catNiveau2 || ''}
                  onChange={(e) => handleCatNiveau2Change(e.target.value)}
                  className="text-xs font-[DM_Sans] bg-slate-100 rounded-lg px-2.5 py-1.5 text-slate-700 border-none outline-none"
                >
                  <option value="">Toutes sous-cat.</option>
                  {catOptions2.map((c) => <option key={c} value={c}>{c}</option>)}
                </select>
              )}

              {catNiveau2 && catOptions3.length > 0 && (
                <select
                  value={categorie || ''}
                  onChange={(e) => handleCategorieChange(e.target.value)}
                  className="text-xs font-[DM_Sans] bg-slate-100 rounded-lg px-2.5 py-1.5 text-slate-700 border-none outline-none"
                >
                  <option value="">Toutes</option>
                  {catOptions3.map((c) => <option key={c} value={c}>{c}</option>)}
                </select>
              )}

              {catNiveau1 && (
                <button
                  onClick={resetCatFilter}
                  className="p-1 rounded-md hover:bg-slate-200 transition-colors"
                  title="Reinitialiser le filtre"
                >
                  <X size={14} className="text-slate-400" />
                </button>
              )}
            </>
          )}
        </div>

        {/* Tab toggle */}
        <div className="flex items-center gap-1 mt-3">
          <div className="flex rounded-lg overflow-hidden bg-slate-100">
            <button
              onClick={() => setActiveTab('synthese')}
              className={`px-3 py-1.5 text-xs font-medium font-[DM_Sans] transition-colors ${
                activeTab === 'synthese' ? 'bg-[#0C419A] text-white' : 'text-slate-500 hover:text-slate-700'
              }`}
            >
              Synthese
            </button>
            <button
              onClick={() => setActiveTab('par-categorie')}
              className={`px-3 py-1.5 text-xs font-medium font-[DM_Sans] transition-colors ${
                activeTab === 'par-categorie' ? 'bg-[#0C419A] text-white' : 'text-slate-500 hover:text-slate-700'
              }`}
            >
              Par categorie
            </button>
          </div>
        </div>
      </header>

      <div className="px-8 pb-8 pt-6 space-y-6">
        {loading && !data && (
          <p className="text-sm text-slate-400">Chargement...</p>
        )}

        {error && (
          <Card>
            <p className="text-sm text-red-600">{error}</p>
          </Card>
        )}

        {activeTab === 'synthese' && data && (
          <>
            {/* KPI Cards */}
            <div className="grid grid-cols-4 gap-5 animate-fade-slide-up">
              <KpiCard
                label="Taux 24h"
                value={data.taux24h}
                format="percent"
                accentColor="#2E7D32"
                icon={<Zap size={18} className="text-emerald-600" />}
              />
              <KpiCard
                label="Taux 48h"
                value={data.taux48h}
                format="percent"
                accentColor="#1565C0"
                icon={<Clock size={18} className="text-blue-600" />}
              />
              <KpiCard
                label="MTTR"
                value={data.mttrJours}
                format="days"
                accentColor="#FF8F00"
                icon={<Timer size={18} className="text-amber-600" />}
              />
              <KpiCard
                label="Mediane"
                value={data.medianeJours}
                format="days"
                accentColor="#6A1B9A"
                icon={<Target size={18} className="text-purple-600" />}
              />
            </div>

            {/* Cumulative treatment rate */}
            <div className="animate-fade-slide-up animation-delay-150">
              <Card>
                <h2 className="text-base font-semibold font-[DM_Sans] text-slate-700 mb-3">
                  Taux de traitement cumule
                </h2>
                <CumulativeBar tranches={data.distribution} />
              </Card>
            </div>

            {/* Trend chart */}
            {data.trend.length > 0 && (
              <div className="animate-fade-slide-up animation-delay-150">
                <Card>
                  <h2 className="text-base font-semibold font-[DM_Sans] text-slate-700 mb-3">
                    Evolution des taux de traitement
                  </h2>
                  <TrendChart data={data} />
                  <p className="text-xs text-slate-400 mt-2 text-center">
                    {data.totalResolus.toLocaleString('fr-FR')} tickets resolus sur la periode
                  </p>
                </Card>
              </div>
            )}

            {/* Distribution chart */}
            <div className="animate-fade-slide-up animation-delay-300">
              <Card>
                <h2 className="text-base font-semibold font-[DM_Sans] text-slate-700 mb-3">
                  Distribution des delais de resolution
                </h2>
                <DistributionChart tranches={data.distribution} />
              </Card>
            </div>

          </>
        )}

        {activeTab === 'par-categorie' && (
          <div className="animate-fade-slide-up">
            <CategorieDelaisTable data={catTableData} loading={catTableLoading} />
          </div>
        )}
      </div>
    </div>
  );
}

export default DelaisPage;
