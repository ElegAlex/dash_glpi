import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { subDays } from 'date-fns';
import { invoke } from '@tauri-apps/api/core';
import * as echarts from 'echarts/core';
import { Clock, Zap, Timer, Target } from 'lucide-react';
import { CompactFilterBar } from '../components/shared/CompactFilterBar';
import { KpiCard } from '../components/shared/KpiCard';
import { Card } from '../components/shared/Card';
import { useInvoke } from '../hooks/useInvoke';
import { useECharts } from '../hooks/useECharts';
import type { DelaisKpi, TrancheDelai } from '../types/delais';
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
      grid: { top: 60, bottom: 40, left: 50, right: 30, containLabel: false },
      xAxis: {
        type: 'category' as const,
        data: labels,
        axisLabel: { rotate: labels.length > 12 ? 35 : 0, fontSize: 11 },
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
        axisLabel: { fontSize: 12, fontFamily: 'DM Sans' },
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

function DelaisPage() {
  const today = new Date();
  const [range, setRange] = useState<DateRange>({ from: subDays(today, 29), to: today });
  const [granularity, setGranularity] = useState<Granularity>('month');
  const { data, loading, error, execute } = useInvoke<DelaisKpi>();
  const initialized = useRef(false);

  const load = useCallback(
    (r: DateRange, g: Granularity) => {
      execute('get_delais_kpi', {
        request: {
          dateFrom: formatDate(r.from),
          dateTo: formatDate(r.to),
          granularity: g,
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
        load({ from, to }, g);
      } else {
        load(range, granularity);
      }
    }).catch(() => {
      load(range, granularity);
    });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleRangeChange = (r: DateRange, autoGran: Granularity) => {
    setRange(r);
    setGranularity(autoGran);
    load(r, autoGran);
  };

  const handleGranularityChange = (g: Granularity) => {
    setGranularity(g);
    load(range, g);
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

        {data && (
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
      </div>
    </div>
  );
}

export default DelaisPage;
