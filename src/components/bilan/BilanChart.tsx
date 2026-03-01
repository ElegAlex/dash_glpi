import { useMemo } from 'react';
import * as echarts from 'echarts/core';
import { DataZoomComponent } from 'echarts/components';
import { useECharts } from '../../hooks/useECharts';
import type { PeriodData } from '../../types/kpi';
import '../../lib/echarts-theme';

echarts.use([DataZoomComponent]);

interface BilanChartProps {
  periodes: PeriodData[];
}

export function BilanChart({ periodes }: BilanChartProps) {
  const option = useMemo(() => {
    const labels = periodes.map((p) => p.periodLabel);
    const entrees = periodes.map((p) => p.entrees);
    const sorties = periodes.map((p) => p.sorties);
    const stockCumule = periodes.map((p) => p.stockCumule ?? null);

    const stockValues = stockCumule.filter((v): v is number => v !== null);
    const hasStock = stockValues.length > 0;
    const maxBar = Math.max(...entrees, ...sorties, 1);
    const maxStock = hasStock ? Math.max(...stockValues, 1) : 1;
    const needSecondaryAxis = hasStock && maxStock > maxBar * 3;

    return {
      tooltip: {
        trigger: 'axis',
        axisPointer: { type: 'cross' },
      },
      legend: {
        data: ['Entrants', 'Sortants', ...(hasStock ? ['Stock net'] : [])],
        top: 8,
      },
      toolbox: {
        feature: {
          saveAsImage: { title: 'PNG', name: 'bilan' },
        },
        right: 16,
        top: 4,
      },
      grid: {
        top: 60,
        bottom: 80,
        left: 60,
        right: needSecondaryAxis ? 60 : 30,
        containLabel: false,
      },
      xAxis: {
        type: 'category',
        data: labels,
        axisLabel: { rotate: labels.length > 12 ? 35 : 0, fontSize: 11 },
      },
      yAxis: [
        {
          type: 'value',
          name: 'Tickets',
          minInterval: 1,
          axisLabel: {
            formatter: (v: number) => v >= 1000 ? `${(v / 1000).toFixed(1)}k` : String(v),
          },
        },
        needSecondaryAxis
          ? { type: 'value', name: 'Stock', position: 'right', splitLine: { show: false } }
          : { type: 'value', show: false },
      ],
      dataZoom: [
        { type: 'slider', bottom: 16, height: 20, start: 0, end: 100 },
        { type: 'inside' },
      ],
      series: [
        {
          name: 'Entrants',
          type: 'bar',
          data: entrees,
          barGap: '10%',
          itemStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: '#1976D2' },
              { offset: 1, color: '#1565C0' },
            ]),
            borderRadius: [6, 6, 0, 0],
          },
          emphasis: {
            itemStyle: {
              color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                { offset: 0, color: '#42A5F5' },
                { offset: 1, color: '#1976D2' },
              ]),
            },
          },
        },
        {
          name: 'Sortants',
          type: 'bar',
          data: sorties,
          itemStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: '#43A047' },
              { offset: 1, color: '#2E7D32' },
            ]),
            borderRadius: [6, 6, 0, 0],
          },
        },
        ...(hasStock
          ? [
              {
                name: 'Stock net',
                type: 'line',
                data: stockCumule,
                yAxisIndex: needSecondaryAxis ? 1 : 0,
                smooth: true,
                symbol: 'circle',
                symbolSize: 7,
                lineStyle: { width: 3, color: '#FF8F00' },
                itemStyle: { color: '#FF8F00', borderColor: '#FFF', borderWidth: 2 },
                areaStyle: {
                  color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                    { offset: 0, color: 'rgba(255,143,0,0.15)' },
                    { offset: 1, color: 'rgba(255,143,0,0.02)' },
                  ]),
                },
              },
            ]
          : []),
      ],
    };
  }, [periodes]);

  const { chartRef } = useECharts(option, undefined, 'cpam-material');

  return <div ref={chartRef} style={{ height: '360px', width: '100%' }} />;
}
