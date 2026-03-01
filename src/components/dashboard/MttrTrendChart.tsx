import { useMemo } from 'react';
import * as echarts from 'echarts/core';
import { useECharts } from '../../hooks/useECharts';
import type { MttrTrend } from '../../types/dashboard';
import '../../lib/echarts-theme';

interface MttrTrendChartProps {
  data: MttrTrend[];
}

export function MttrTrendChart({ data }: MttrTrendChartProps) {
  const option = useMemo(() => {
    const labels = data.map((d) => d.periode);
    const mttr = data.map((d) => d.mttrJours);
    const mediane = data.map((d) => d.medianeJours);

    return {
      tooltip: {
        trigger: 'axis' as const,
        formatter: (params: Array<{ seriesName: string; value: number; marker: string; dataIndex: number }>) => {
          const idx = params[0]?.dataIndex ?? 0;
          const item = data[idx];
          let html = `<div style="font-weight:600;margin-bottom:4px">${item?.periode ?? ''}</div>`;
          for (const p of params) {
            html += `<div>${p.marker} ${p.seriesName}: <b>${p.value.toFixed(1)}j</b></div>`;
          }
          if (item) {
            html += `<div style="margin-top:4px;color:#94A3B8">Resolus: ${item.nbResolus.toLocaleString('fr-FR')}</div>`;
          }
          return html;
        },
      },
      legend: {
        data: ['MTTR moyen', 'Mediane'],
        top: 8,
      },
      grid: {
        left: 50,
        right: 20,
        top: 50,
        bottom: 30,
      },
      xAxis: {
        type: 'category' as const,
        data: labels,
        axisLabel: { fontSize: 11 },
      },
      yAxis: {
        type: 'value' as const,
        name: 'Jours',
        minInterval: 1,
      },
      series: [
        {
          name: 'MTTR moyen',
          type: 'line' as const,
          data: mttr,
          smooth: true,
          lineStyle: { width: 2.5, color: '#FF8F00' },
          itemStyle: { color: '#FF8F00', borderColor: '#FFF', borderWidth: 2 },
          symbolSize: 7,
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(245,158,11,0.08)' },
              { offset: 1, color: 'rgba(245,158,11,0.01)' },
            ]),
          },
        },
        {
          name: 'Mediane',
          type: 'line' as const,
          data: mediane,
          smooth: true,
          lineStyle: { width: 2, type: 'dashed' as const, color: '#6A1B9A' },
          itemStyle: { color: '#6A1B9A', borderColor: '#FFF', borderWidth: 2 },
          symbolSize: 5,
        },
      ],
    };
  }, [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');

  return <div ref={chartRef} style={{ height: 320, width: '100%' }} />;
}
