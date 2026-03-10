import { useMemo } from 'react';
import * as echarts from 'echarts/core';
import { useECharts } from '../../hooks/useECharts';
import type { ResolutionSpeedTrend } from '../../types/dashboard';
import '../../lib/echarts-theme';

interface ResolutionSpeedTrendChartProps {
  data: ResolutionSpeedTrend[];
  seuil?: number;
}

export function ResolutionSpeedTrendChart({ data, seuil = 80 }: ResolutionSpeedTrendChartProps) {
  const option = useMemo(() => {
    const labels = data.map((d) => d.periode);
    const pct24 = data.map((d) => d.pct24h);
    const pct48 = data.map((d) => d.pct48h);

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
        data: ['< 24h', '< 48h'],
        top: 8,
        textStyle: { fontSize: 8 },
      },
      grid: {
        left: 50,
        right: 20,
        top: 50,
        bottom: 30,
        containLabel: true,
      },
      xAxis: {
        type: 'category' as const,
        data: labels,
        axisLabel: { rotate: labels.length > 6 ? 90 : 0, fontSize: 10 },
      },
      yAxis: {
        type: 'value' as const,
        name: '%',
        nameTextStyle: { fontSize: 10 },
        min: 0,
        max: 100,
        axisLabel: { fontSize: 10 },
      },
      series: [
        {
          name: '< 48h',
          type: 'line' as const,
          data: pct48,
          smooth: true,
          lineStyle: { width: 2, type: 'dashed' as const, color: '#2E7D32' },
          itemStyle: { color: '#2E7D32', borderColor: '#FFF', borderWidth: 2 },
          symbolSize: 5,
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(46,125,50,0.08)' },
              { offset: 1, color: 'rgba(46,125,50,0.01)' },
            ]),
          },
        },
        {
          name: '< 24h',
          type: 'line' as const,
          data: pct24,
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
                yAxis: seuil,
                label: {
                  formatter: `Objectif: ${seuil}%`,
                  position: 'insideEndTop' as const,
                  color: '#C62828',
                  fontSize: 11,
                  fontWeight: 600,
                },
              },
            ],
          },
        },
      ],
    };
  }, [data, seuil]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');

  return <div ref={chartRef} style={{ height: 320, width: '100%' }} />;
}
