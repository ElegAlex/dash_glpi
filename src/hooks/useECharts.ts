import { useRef, useEffect, useCallback } from 'react';
import type { ECharts, EChartsCoreOption } from 'echarts/core';
import * as echarts from 'echarts/core';
import { TreemapChart, SunburstChart, HeatmapChart, SankeyChart, LineChart, BarChart } from 'echarts/charts';
import {
  TooltipComponent,
  VisualMapComponent,
  LegendComponent,
  ToolboxComponent,
  GridComponent,
  TitleComponent,
} from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';
import { UniversalTransition } from 'echarts/features';

echarts.use([
  TreemapChart, SunburstChart, HeatmapChart, SankeyChart, LineChart, BarChart,
  TooltipComponent, VisualMapComponent, LegendComponent,
  ToolboxComponent, GridComponent, TitleComponent,
  CanvasRenderer, UniversalTransition,
]);

const CHART_PALETTE = [
  '#0C419A', '#E69F00', '#009E73', '#D55E00', '#56B4E9',
  '#CC79A7', '#0072B2', '#228833', '#EE6677', '#AA3377', '#4477AA',
];

echarts.registerTheme('cpam', {
  color: CHART_PALETTE,
  backgroundColor: 'transparent',
  textStyle: { fontFamily: 'Inter, Segoe UI, system-ui, sans-serif', fontSize: 13, color: '#1a1f2e' },
  tooltip: {
    backgroundColor: '#ffffff',
    borderColor: '#e2e6ee',
    borderWidth: 1,
    textStyle: { color: '#1a1f2e', fontSize: 13 },
  },
  categoryAxis: {
    axisLine: { lineStyle: { color: '#cdd3df' } },
    axisLabel: { color: '#525d73', fontSize: 12 },
    splitLine: { lineStyle: { color: '#f1f3f7' } },
  },
  valueAxis: {
    axisLine: { lineStyle: { color: '#cdd3df' } },
    axisLabel: { color: '#525d73', fontSize: 12 },
    splitLine: { lineStyle: { color: '#f1f3f7', type: 'dashed' } },
  },
});

export function useECharts(
  option: EChartsCoreOption,
  onEvents?: Record<string, (params: unknown, chart: ECharts) => void>,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<ECharts | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const chart = echarts.init(containerRef.current, 'cpam', { renderer: 'canvas' });
    chartRef.current = chart;

    const resizeObserver = new ResizeObserver(() => chart.resize());
    resizeObserver.observe(containerRef.current);

    return () => {
      resizeObserver.disconnect();
      chart.dispose();
      chartRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!chartRef.current) return;
    chartRef.current.setOption(option, { notMerge: true });
  }, [option]);

  useEffect(() => {
    if (!chartRef.current || !onEvents) return;
    const chart = chartRef.current;
    Object.entries(onEvents).forEach(([event, handler]) => {
      chart.on(event, (params) => handler(params, chart));
    });
    return () => {
      Object.keys(onEvents).forEach((event) => chart.off(event));
    };
  }, [onEvents]);

  const setOption = useCallback((opt: EChartsCoreOption) => {
    chartRef.current?.setOption(opt, { notMerge: true });
  }, []);

  return { chartRef: containerRef, setOption };
}
