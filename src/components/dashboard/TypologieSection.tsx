import { useMemo } from 'react';
import { useECharts } from '../../hooks/useECharts';
import { Card } from '../shared/Card';
import type { TypologieKpi, VentilationItem } from '../../types/dashboard';
import '../../lib/echarts-theme';

const PALETTE = ['#1565C0', '#2E7D32', '#FF8F00', '#6A1B9A', '#00838F', '#C62828', '#4E342E', '#37474F'];

interface TypologieSectionProps {
  typologie: TypologieKpi;
}

function TypePieChart({ data }: { data: VentilationItem[] }) {
  const option = useMemo(() => ({
    tooltip: {
      trigger: 'item' as const,
      formatter: (params: { name: string; value: number; percent: number; marker: string }) =>
        `<div style="font-weight:600;margin-bottom:4px">${params.name}</div>
         <div>${params.marker} Total: <b>${params.value.toLocaleString('fr-FR')}</b> (${params.percent.toFixed(1)}%)</div>`,
    },
    legend: {
      orient: 'vertical' as const,
      right: 10,
      top: 'center' as const,
      textStyle: { fontSize: 12 },
    },
    series: [
      {
        type: 'pie' as const,
        radius: ['40%', '70%'],
        center: ['35%', '50%'],
        avoidLabelOverlap: true,
        itemStyle: {
          borderRadius: 6,
          borderColor: '#fff',
          borderWidth: 2,
        },
        label: { show: false },
        emphasis: {
          label: {
            show: true,
            fontSize: 14,
            fontWeight: 'bold' as const,
          },
        },
        data: data.map((item, idx) => ({
          value: item.total,
          name: item.label,
          itemStyle: { color: PALETTE[idx % PALETTE.length] },
        })),
      },
    ],
  }), [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');
  return <div ref={chartRef} style={{ height: 280, width: '100%' }} />;
}

function PrioriteBarChart({ data }: { data: VentilationItem[] }) {
  const PRIORITY_COLORS: Record<string, string> = {
    'Tres haute': '#C62828',
    'Haute': '#FF8F00',
    'Moyenne': '#FFC107',
    'Basse': '#2E7D32',
    'Tres basse': '#66BB6A',
  };

  const option = useMemo(() => {
    const labels = data.map((d) => d.label);
    const values = data.map((d) => d.total);

    return {
      tooltip: {
        trigger: 'axis' as const,
        axisPointer: { type: 'shadow' as const },
      },
      grid: {
        left: 100,
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
        axisLabel: { fontSize: 11 },
      },
      series: [
        {
          type: 'bar' as const,
          data: values.map((val, idx) => ({
            value: val,
            itemStyle: {
              color: PRIORITY_COLORS[labels[idx]] ?? PALETTE[idx % PALETTE.length],
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

export function TypologieSection({ typologie }: TypologieSectionProps) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
      <Card>
        <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 mb-3">
          Repartition par type
        </h3>
        <TypePieChart data={typologie.parType} />
      </Card>
      <Card>
        <h3 className="text-sm font-semibold font-[DM_Sans] text-slate-700 mb-3">
          Repartition par priorite
        </h3>
        <PrioriteBarChart data={typologie.parPriorite} />
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
