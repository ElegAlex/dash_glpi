import * as echarts from 'echarts/core';

export const CPAM_MATERIAL_THEME: Record<string, unknown> = {
  color: [
    '#1565C0', '#2E7D32', '#FF8F00', '#6A1B9A',
    '#00838F', '#C62828', '#4E342E', '#37474F',
  ],
  backgroundColor: 'transparent',

  title: {
    textStyle: {
      fontFamily: 'DM Sans, system-ui',
      fontWeight: 700,
      fontSize: 16,
      color: '#1E293B',
    },
    subtextStyle: {
      fontFamily: 'Source Sans 3, system-ui',
      fontSize: 13,
      color: '#64748B',
    },
  },

  legend: {
    textStyle: {
      fontFamily: 'Source Sans 3, system-ui',
      fontSize: 13,
      color: '#64748B',
    },
    icon: 'roundRect',
    itemWidth: 14,
    itemHeight: 8,
    itemGap: 20,
  },

  tooltip: {
    backgroundColor: '#FFFFFF',
    borderColor: '#E2E8F0',
    borderWidth: 1,
    textStyle: {
      fontFamily: 'Source Sans 3, system-ui',
      fontSize: 13,
      color: '#1E293B',
    },
    extraCssText: `
      border-radius: 12px;
      box-shadow: 0 10px 20px rgba(0,0,0,0.10), 0 3px 6px rgba(0,0,0,0.06);
      padding: 12px 16px;
    `,
  },

  categoryAxis: {
    axisLine: { lineStyle: { color: '#E2E8F0' } },
    axisTick: { show: false },
    axisLabel: {
      fontFamily: 'Source Sans 3, system-ui',
      fontSize: 12,
      color: '#94A3B8',
    },
    splitLine: { show: false },
  },

  valueAxis: {
    axisLine: { show: false },
    axisTick: { show: false },
    axisLabel: {
      fontFamily: 'Source Sans 3, system-ui',
      fontSize: 12,
      color: '#94A3B8',
    },
    splitLine: {
      lineStyle: { color: '#F1F5F9', type: 'dashed' },
    },
  },

  bar: {
    barBorderRadius: [6, 6, 0, 0],
    itemStyle: { borderRadius: [6, 6, 0, 0] },
  },

  line: {
    smooth: true,
    symbolSize: 6,
    lineStyle: { width: 2.5 },
  },
};

echarts.registerTheme('cpam-material', CPAM_MATERIAL_THEME);
