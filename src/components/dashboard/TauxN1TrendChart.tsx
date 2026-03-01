import { useMemo } from 'react';
import * as echarts from 'echarts/core';
import { useECharts } from '../../hooks/useECharts';
import type { TauxN1Trend } from '../../types/dashboard';
import '../../lib/echarts-theme';

interface TauxN1TrendChartProps {
  data: TauxN1Trend[];
  objectifItil: number;
}

export function TauxN1TrendChart({ data, objectifItil }: TauxN1TrendChartProps) {
  const option = useMemo(() => {
    const labels = data.map((d) => d.periode);
    const n1Strict = data.map((d) => d.n1StrictPct);
    const n1Elargi = data.map((d) => d.n1ElargiPct);

    return {
      tooltip: {
        trigger: 'axis' as const,
        formatter: (params: Array<{ seriesName: string; value: number; marker: string; dataIndex: number }>) => {
          const idx = params[0]?.dataIndex ?? 0;
          const item = data[idx];
          let html = `<div style="font-weight:600;margin-bottom:4px">${item?.periode ?? ''}</div>`;
          for (const p of params) {
            html += `<div>${p.marker} ${p.seriesName}: <b>${p.value.toFixed(1)}%</b></div>`;
          }
          if (item) {
            html += `<div style="margin-top:4px;color:#94A3B8">Resolus: ${item.totalResolus.toLocaleString('fr-FR')}</div>`;
          }
          return html;
        },
      },
      legend: {
        data: ['N1 strict', 'N1 elargi'],
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
        name: '%',
        min: 0,
        max: 100,
      },
      series: [
        {
          name: 'N1 strict',
          type: 'line' as const,
          data: n1Strict,
          smooth: true,
          lineStyle: { width: 2.5, color: '#1565C0' },
          itemStyle: { color: '#1565C0', borderColor: '#FFF', borderWidth: 2 },
          symbolSize: 7,
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(21,101,192,0.10)' },
              { offset: 1, color: 'rgba(21,101,192,0.01)' },
            ]),
          },
          markLine: {
            silent: true,
            symbol: 'none',
            lineStyle: { type: 'dotted' as const, color: '#C62828', width: 2 },
            data: [
              {
                yAxis: objectifItil,
                label: {
                  formatter: `Objectif ITIL: ${objectifItil}%`,
                  position: 'insideEndTop' as const,
                  color: '#C62828',
                  fontSize: 11,
                  fontWeight: 600,
                },
              },
            ],
          },
        },
        {
          name: 'N1 elargi',
          type: 'line' as const,
          data: n1Elargi,
          smooth: true,
          lineStyle: { width: 2, type: 'dashed' as const, color: '#2E7D32' },
          itemStyle: { color: '#2E7D32', borderColor: '#FFF', borderWidth: 2 },
          symbolSize: 5,
        },
      ],
    };
  }, [data, objectifItil]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');

  return <div ref={chartRef} style={{ height: 320, width: '100%' }} />;
}
