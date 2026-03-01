import { useMemo } from 'react';
import * as echarts from 'echarts/core';
import { DataZoomComponent } from 'echarts/components';
import { useECharts } from '../../hooks/useECharts';
import type { PeriodData } from '../../types/kpi';

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
        top: 56,
        bottom: 80,
        left: 56,
        right: needSecondaryAxis ? 60 : 24,
      },
      xAxis: {
        type: 'category',
        data: labels,
        axisLabel: { rotate: labels.length > 12 ? 45 : 0 },
      },
      yAxis: [
        { type: 'value', name: 'Tickets', minInterval: 1 },
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
          itemStyle: { color: '#0C419A' },
          barGap: '10%',
        },
        {
          name: 'Sortants',
          type: 'bar',
          data: sorties,
          itemStyle: { color: '#009E73' },
        },
        ...(hasStock
          ? [
              {
                name: 'Stock net',
                type: 'line',
                data: stockCumule,
                yAxisIndex: needSecondaryAxis ? 1 : 0,
                smooth: true,
                itemStyle: { color: '#E69F00' },
                lineStyle: { width: 2 },
                symbol: 'circle',
                symbolSize: 5,
              },
            ]
          : []),
      ],
    };
  }, [periodes]);

  const { chartRef } = useECharts(option);

  return <div ref={chartRef} style={{ height: '360px', width: '100%' }} />;
}
