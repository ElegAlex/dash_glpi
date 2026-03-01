import { useMemo } from 'react';
import { useECharts } from '../../hooks/useECharts';
import { Card } from '../shared/Card';
import type { TypologieKpi, VentilationItem, VolumePeriode } from '../../types/dashboard';
import '../../lib/echarts-theme';

const PALETTE = ['#1565C0', '#2E7D32', '#FF8F00', '#6A1B9A', '#00838F', '#C62828', '#4E342E', '#37474F'];

interface TypologieSectionProps {
  typologie: TypologieKpi;
  volumes: VolumePeriode[];
}

function SoldeEvolutionChart({ data }: { data: VolumePeriode[] }) {
  const option = useMemo(() => {
    const periodes = data.map((d) => d.periode);

    // Cumulative balance
    let cumul = 0;
    const soldeCumule = data.map((d) => {
      cumul += d.crees - d.resolus;
      return cumul;
    });

    return {
      tooltip: {
        trigger: 'axis' as const,
        formatter: (params: { name: string; value: number; marker: string }[]) => {
          const p = params[0];
          const idx = periodes.indexOf(p.name);
          const d = data[idx];
          return `<div style="font-weight:600;margin-bottom:4px">${p.name}</div>
                  <div>Crees: <b>${d.crees.toLocaleString('fr-FR')}</b></div>
                  <div>Resolus: <b>${d.resolus.toLocaleString('fr-FR')}</b></div>
                  <div>Delta: <b>${(d.crees - d.resolus) > 0 ? '+' : ''}${(d.crees - d.resolus).toLocaleString('fr-FR')}</b></div>
                  <div>${p.marker} Solde cumule: <b>${p.value > 0 ? '+' : ''}${p.value.toLocaleString('fr-FR')}</b></div>`;
        },
      },
      grid: { left: 50, right: 20, top: 20, bottom: 30 },
      xAxis: {
        type: 'category' as const,
        data: periodes,
        axisLabel: { fontSize: 10, rotate: periodes.length > 12 ? 45 : 0 },
      },
      yAxis: {
        type: 'value' as const,
        splitLine: { lineStyle: { type: 'dashed' as const, color: '#F1F5F9' } },
      },
      series: [
        {
          type: 'line' as const,
          data: soldeCumule,
          smooth: true,
          lineStyle: { width: 3, color: '#FF8F00' },
          itemStyle: {
            color: '#FF8F00',
            borderColor: '#fff',
            borderWidth: 2,
          },
          areaStyle: {
            color: {
              type: 'linear' as const,
              x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: 'rgba(255,143,0,0.15)' },
                { offset: 1, color: 'rgba(255,143,0,0.02)' },
              ],
            },
          },
          markLine: {
            silent: true,
            symbol: 'none',
            lineStyle: { type: 'dashed' as const, color: '#94A3B8', width: 1 },
            data: [{ yAxis: 0 }],
            label: { show: false },
          },
        },
      ],
    };
  }, [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');
  return <div ref={chartRef} style={{ height: 280, width: '100%' }} />;
}

function GroupeBarChart({ data }: { data: VentilationItem[] }) {
  const option = useMemo(() => {
    const top10 = [...data].sort((a, b) => b.total - a.total).slice(0, 10);
    const labels = top10.map((d) => d.label);
    const values = top10.map((d) => d.total);

    return {
      tooltip: {
        trigger: 'axis' as const,
        axisPointer: { type: 'shadow' as const },
      },
      grid: {
        left: 140,
        right: 30,
        top: 10,
        bottom: 20,
      },
      xAxis: {
        type: 'value' as const,
      },
      yAxis: {
        type: 'category' as const,
        data: labels,
        inverse: true,
        axisLabel: {
          fontSize: 11,
          width: 120,
          overflow: 'truncate' as const,
        },
      },
      series: [
        {
          type: 'bar' as const,
          data: values.map((val, idx) => ({
            value: val,
            itemStyle: {
              color: PALETTE[idx % PALETTE.length],
              borderRadius: [0, 6, 6, 0],
            },
          })),
          barWidth: '60%',
          label: {
            show: true,
            position: 'right' as const,
            fontSize: 11,
            color: '#64748B',
            formatter: (params: { value: number }) => params.value.toLocaleString('fr-FR'),
          },
        },
      ],
    };
  }, [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');
  return <div ref={chartRef} style={{ height: 280, width: '100%' }} />;
}

export function TypologieSection({ typologie, volumes }: TypologieSectionProps) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
      <Card>
        <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 mb-3">
          Evolution du solde (crees - resolus)
        </h3>
        <SoldeEvolutionChart data={volumes} />
      </Card>
      <Card>
        <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 mb-3">
          Top 10 groupes
        </h3>
        <GroupeBarChart data={typologie.parGroupe} />
      </Card>
    </div>
  );
}
