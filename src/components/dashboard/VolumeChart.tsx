import { useMemo } from 'react';
import * as echarts from 'echarts/core';
import { useECharts } from '../../hooks/useECharts';
import type { VolumePeriode } from '../../types/dashboard';
import '../../lib/echarts-theme';

interface VolumeChartProps {
  data: VolumePeriode[];
}

export function VolumeChart({ data }: VolumeChartProps) {
  const option = useMemo(() => {
    const labels = data.map((d) => d.periode);
    const crees = data.map((d) => d.crees);
    const resolus = data.map((d) => d.resolus);

    return {
      tooltip: {
        trigger: 'axis' as const,
        axisPointer: { type: 'shadow' as const },
        formatter: (params: Array<{ seriesName: string; value: number; marker: string }>) => {
          const idx = params[0] ? data.findIndex((d) => d.periode === labels[params[0].value as unknown as number]) : -1;
          const period = params.length > 0 ? labels[(params[0].value as unknown as number)] : '';
          let html = `<div style="font-weight:600;margin-bottom:4px">${period || params[0]?.seriesName}</div>`;
          for (const p of params) {
            html += `<div>${p.marker} ${p.seriesName}: <b>${p.value.toLocaleString('fr-FR')}</b></div>`;
          }
          const delta = idx >= 0 ? data[idx].delta : (params.length === 2 ? params[1].value - params[0].value : 0);
          const color = delta >= 0 ? '#2E7D32' : '#C62828';
          html += `<div style="margin-top:4px;color:${color};font-weight:600">Delta: ${delta >= 0 ? '+' : ''}${delta.toLocaleString('fr-FR')}</div>`;
          return html;
        },
      },
      legend: {
        data: ['Crees', 'Resolus'],
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
        axisLabel: { rotate: labels.length > 12 ? 35 : 0, fontSize: 11 },
      },
      yAxis: {
        type: 'value' as const,
        name: 'Tickets',
        minInterval: 1,
      },
      series: [
        {
          name: 'Crees',
          type: 'bar' as const,
          data: crees,
          barGap: '10%',
          itemStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: '#1976D2' },
              { offset: 1, color: '#1565C0' },
            ]),
            borderRadius: [6, 6, 0, 0],
          },
        },
        {
          name: 'Resolus',
          type: 'bar' as const,
          data: resolus,
          itemStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: '#43A047' },
              { offset: 1, color: '#2E7D32' },
            ]),
            borderRadius: [6, 6, 0, 0],
          },
        },
      ],
    };
  }, [data]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');

  return <div ref={chartRef} style={{ height: 320, width: '100%' }} />;
}
