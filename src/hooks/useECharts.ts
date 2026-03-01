import { useRef, useEffect, useCallback } from 'react';
import type { ECharts, EChartsCoreOption } from 'echarts/core';
import * as echarts from 'echarts/core';
import { TreemapChart, SunburstChart, HeatmapChart, SankeyChart, LineChart, BarChart, GraphChart, PieChart } from 'echarts/charts';
import {
  TooltipComponent,
  VisualMapComponent,
  LegendComponent,
  ToolboxComponent,
  GridComponent,
  TitleComponent,
  MarkLineComponent,
} from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';
import { UniversalTransition } from 'echarts/features';

echarts.use([
  TreemapChart, SunburstChart, HeatmapChart, SankeyChart, LineChart, BarChart, GraphChart, PieChart,
  TooltipComponent, VisualMapComponent, LegendComponent,
  ToolboxComponent, GridComponent, TitleComponent, MarkLineComponent,
  CanvasRenderer, UniversalTransition,
]);


export function useECharts(
  option: EChartsCoreOption,
  onEvents?: Record<string, (params: unknown, chart: ECharts) => void>,
  theme?: string,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<ECharts | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const chart = echarts.init(containerRef.current, theme ?? 'cpam-material', { renderer: 'canvas' });
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
