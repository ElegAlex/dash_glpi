import { useMemo } from 'react';
import { useECharts } from '../../hooks/useECharts';
import type { TrancheDelai } from '../../types/dashboard';
import '../../lib/echarts-theme';

interface DistributionDelaisChartProps {
  data: TrancheDelai[];
}

const GRADIENT_COLORS = ['#2E7D32', '#66BB6A', '#FFC107', '#FF8F00', '#C62828'];

export function DistributionDelaisChart({ data }: DistributionDelaisChartProps) {
  const option = useMemo(() => {
    const labels = data.map((d) => d.label);
    const counts = data.map((d) => d.count);
    const percentages = data.map((d) => d.pourcentage);

    return {
      tooltip: {
        trigger: 'axis' as const,
        axisPointer: { type: 'shadow' as const },
        formatter: (params: Array<{ name: string; value: number; dataIndex: number; marker: string }>) => {
          const p = params[0];
          if (!p) return '';
          const pct = percentages[p.dataIndex] ?? 0;
          return `<div style="font-weight:600;margin-bottom:4px">${p.name}</div>
                  <div>${p.marker} Tickets: <b>${p.value.toLocaleString('fr-FR')}</b></div>
                  <div style="color:#64748B">${pct.toFixed(1)}%</div>`;
        },
      },
      grid: {
        left: 120,
        right: 60,
        top: 20,
        bottom: 20,
      },
      xAxis: {
        type: 'value' as const,
        name: 'Tickets',
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
          data: counts.map((val, idx) => ({
            value: val,
            itemStyle: {
              color: GRADIENT_COLORS[idx % GRADIENT_COLORS.length],
              borderRadius: [0, 6, 6, 0],
            },
            label: {
              show: true,
              position: 'right' as const,
              formatter: `${percentages[idx]?.toFixed(1) ?? '0'}%`,
              fontSize: 11,
              color: '#64748B',
            },
          })),
          barWidth: '60%',
        },
      ],
    };
  }, [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');

  return <div ref={chartRef} style={{ height: 320, width: '100%' }} />;
}
