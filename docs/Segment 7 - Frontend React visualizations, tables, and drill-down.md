# Segment 7 - Frontend React visualizations, tables, and drill-down

This segment provides the complete frontend technology stack for the CPAM GLPI Dashboard — a Tauri 2.10+ desktop application built with React 19, TypeScript, and Tailwind CSS 4. **Apache ECharts 6 emerges as the primary visualization library**, TanStack Table v8 powers the data grid with virtual scrolling for 10,000+ rows, and a CSS-first Tailwind v4 theme delivers an accessible, institutional design. Every library recommendation below has been validated for React 19 compatibility, TypeScript-first development, and SSR-free desktop deployment.

---

## 1. ECharts wins the charting library showdown

Three libraries were evaluated for the four required chart types (treemap, sunburst, heatmap, Sankey) in a Tauri desktop context where SSR is irrelevant and performance with large datasets matters most.

### Comparison matrix

|Feature|Apache ECharts 6.0|Recharts 3.7|Nivo 0.99|
|---|---|---|---|
|**Treemap**|⭐⭐⭐⭐⭐ Native, built-in drill-down|✅ Basic|⭐⭐⭐⭐ SVG/HTML/Canvas|
|**Sunburst**|⭐⭐⭐⭐⭐ Native, drill-down default|✅ Basic (v3+)|⭐⭐⭐⭐ SVG only|
|**Heatmap**|⭐⭐⭐⭐⭐ Native, VisualMap|❌ Not supported|⭐⭐⭐⭐ SVG/Canvas|
|**Sankey**|⭐⭐⭐⭐⭐ Native, draggable nodes|✅ Basic|⭐⭐⭐⭐ SVG only|
|**10K+ points**|⭐⭐⭐⭐⭐ Canvas/WebGL, 100K+ smooth|⭐⭐ SVG only, lags at 8K|⭐⭐⭐ Canvas where available|
|**Bundle (gzip)**|~100–150 KB tree-shaken|~95–110 KB|~90–135 KB (4 charts + core)|
|**React 19**|✅ Framework-agnostic|✅ With `react-is` override|✅ Since v0.98.0|
|**TypeScript**|⭐⭐⭐⭐⭐ `ComposeOption<>` generics|⭐⭐⭐⭐ Native in v3|⭐⭐⭐⭐½ Native|
|**Drill-down**|✅ Built-in (`groupId`, `leafDepth`)|❌ Manual|❌ Manual|
|**Rendering**|Canvas + SVG + WebGL|SVG only|SVG + Canvas + HTML|
|**GitHub stars**|~62K|~26K|~13.5K|

**ECharts is the decisive winner** for this project. All four chart types are first-class citizens with the richest configuration options. Canvas rendering by default handles **100,000+ data points** without breaking a sweat — critical for a desktop app analyzing potentially thousands of GLPI tickets across heatmaps and Sankey diagrams. The built-in drill-down system with animated transitions eliminates weeks of custom implementation. Recharts is disqualified by its missing heatmap and poor large-dataset performance. Nivo is a solid runner-up with a more "React-native" declarative API, but lacks built-in drill-down and Canvas rendering for sunburst/Sankey.

### Tree-shaking configuration

ECharts 6 supports granular imports via `echarts/core`. For the CPAM dashboard requiring exactly four chart types, the tree-shaken bundle lands at approximately **100–150 KB gzipped** versus ~320 KB for a full import:

```typescript
import * as echarts from 'echarts/core';
import { TreemapChart, SunburstChart, HeatmapChart, SankeyChart, LineChart } from 'echarts/charts';
import {
  TooltipComponent, VisualMapComponent, LegendComponent,
  ToolboxComponent, BrushComponent, DataZoomComponent, GridComponent,
} from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';
import { UniversalTransition } from 'echarts/features';

echarts.use([
  TreemapChart, SunburstChart, HeatmapChart, SankeyChart, LineChart,
  TooltipComponent, VisualMapComponent, LegendComponent,
  ToolboxComponent, BrushComponent, DataZoomComponent, GridComponent,
  CanvasRenderer, UniversalTransition,
]);

// Type-safe option construction
import type { TreemapSeriesOption, SunburstSeriesOption, HeatmapSeriesOption, SankeySeriesOption }
  from 'echarts/charts';
import type { TooltipComponentOption, VisualMapComponentOption } from 'echarts/components';
import type { ComposeOption } from 'echarts/core';

type ECOption = ComposeOption<
  TreemapSeriesOption | SunburstSeriesOption | HeatmapSeriesOption | SankeySeriesOption |
  TooltipComponentOption | VisualMapComponentOption
>;
```

### React wrapper recommendation

The `echarts-for-react` package (v3.0.6) works but has not seen significant source updates since 2021. For a production Tauri app, **a custom wrapper of ~50 lines provides more control** and eliminates a third-party dependency:

```typescript
import { useRef, useEffect, useCallback } from 'react';
import type { ECharts, SetOptionOpts } from 'echarts/core';
import * as echarts from 'echarts/core';

interface UseEChartsOptions {
  option: echarts.EChartsOption;
  theme?: string | object;
  onEvents?: Record<string, (params: any, chart: ECharts) => void>;
  notMerge?: boolean;
  lazyUpdate?: boolean;
}

export function useECharts({ option, theme, onEvents, notMerge = true, lazyUpdate = false }: UseEChartsOptions) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<ECharts | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const chart = echarts.init(containerRef.current, theme, { renderer: 'canvas' });
    chartRef.current = chart;

    const resizeObserver = new ResizeObserver(() => chart.resize());
    resizeObserver.observe(containerRef.current);

    return () => {
      resizeObserver.disconnect();
      chart.dispose();
      chartRef.current = null;
    };
  }, [theme]);

  useEffect(() => {
    if (!chartRef.current) return;
    chartRef.current.setOption(option, { notMerge, lazyUpdate } as SetOptionOpts);
  }, [option, notMerge, lazyUpdate]);

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

  return { containerRef, chartInstance: chartRef };
}
```

Usage in a component:

```tsx
function TreemapChart({ data }: { data: TreeNode[] }) {
  const option = useMemo<ECOption>(() => ({
    series: [{ type: 'treemap', data, leafDepth: 1, breadcrumb: { show: true } }],
  }), [data]);

  const onEvents = useMemo(() => ({
    click: (params: any) => console.log('Clicked:', params.treePathInfo),
  }), []);

  const { containerRef } = useECharts({ option, theme: 'cpam', onEvents });
  return <div ref={containerRef} className="h-96 w-full" />;
}
```

---

## 2. TanStack Table v8 with virtualized scrolling

**@tanstack/react-table v8.21.3** is the production-stable version for React 19. A v9 alpha exists (v9.0.0-alpha.11) but remains unstable. Combined with **@tanstack/react-virtual v3.13.19** (~3.9 KB gzipped), this pairing delivers smooth 60 FPS scrolling across 50,000+ rows with only ~30–50 DOM nodes rendered at any time.

### GLPI ticket type definitions and column setup

```typescript
import { createColumnHelper, type ColumnDef } from '@tanstack/react-table';

// Core GLPI ticket shape matching the Rust backend export
interface GLPITicket {
  id: number;
  title: string;
  status: 'new' | 'processing' | 'pending' | 'solved' | 'closed';
  priority: 1 | 2 | 3 | 4 | 5;
  urgency: 1 | 2 | 3 | 4 | 5;
  technician: string;
  group: string;            // e.g. "_DSI > _SUPPORT > _PARC"
  category: string;         // hierarchical: "Cat1 > Cat2 > Cat3"
  requester: string;
  dateCreated: string;      // ISO 8601
  dateSolved: string | null;
  dateClosed: string | null;
  ageInDays: number;        // computed by Rust backend
}

const columnHelper = createColumnHelper<GLPITicket>();

const columns = [
  columnHelper.display({
    id: 'select',
    header: ({ table }) => (
      <input type="checkbox"
        checked={table.getIsAllRowsSelected()}
        indeterminate={table.getIsSomeRowsSelected()}
        onChange={table.getToggleAllRowsSelectedHandler()} />
    ),
    cell: ({ row }) => (
      <input type="checkbox"
        checked={row.getIsSelected()}
        onChange={row.getToggleSelectedHandler()} />
    ),
    size: 40,
  }),
  columnHelper.accessor('id', {
    header: 'ID',
    cell: (info) => `#${info.getValue()}`,
    size: 70,
  }),
  columnHelper.accessor('title', {
    header: 'Titre',
    size: 300,
  }),
  columnHelper.accessor('status', {
    header: 'Statut',
    filterFn: 'equals',
    size: 100,
    cell: (info) => <StatusBadge status={info.getValue()} />,
  }),
  columnHelper.accessor('priority', {
    header: 'Priorité',
    sortingFn: 'basic',
    size: 80,
  }),
  columnHelper.accessor('technician', { header: 'Technicien', size: 150 }),
  columnHelper.accessor('group', {
    header: 'Groupe',
    enableGrouping: true,
    size: 200,
  }),
  columnHelper.accessor('category', {
    header: 'Catégorie',
    enableGrouping: true,
    size: 200,
  }),
  columnHelper.accessor('dateCreated', {
    header: 'Créé le',
    sortingFn: 'datetime',
    cell: (info) => new Date(info.getValue()).toLocaleDateString('fr-FR'),
    size: 110,
  }),
  columnHelper.accessor('ageInDays', {
    header: 'Âge (j)',
    sortingFn: 'basic',
    size: 80,
    cell: (info) => <AgeBadge days={info.getValue()} />,
  }),
  columnHelper.display({
    id: 'actions',
    header: 'Actions',
    cell: ({ row }) => <RowActions ticket={row.original} />,
    size: 80,
  }),
];
```

### Table instance with all features

```typescript
import {
  useReactTable, getCoreRowModel, getSortedRowModel,
  getFilteredRowModel, getGroupedRowModel, getExpandedRowModel,
  getFacetedRowModel, getFacetedUniqueValues,
  type SortingState, type ColumnFiltersState,
  type GroupingState, type RowSelectionState,
} from '@tanstack/react-table';

const [sorting, setSorting] = useState<SortingState>([]);
const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
const [grouping, setGrouping] = useState<GroupingState>([]);
const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
const [globalFilter, setGlobalFilter] = useState('');

const table = useReactTable({
  data: tickets,
  columns,
  state: { sorting, columnFilters, grouping, rowSelection, globalFilter },
  onSortingChange: setSorting,
  onColumnFiltersChange: setColumnFilters,
  onGroupingChange: setGrouping,
  onRowSelectionChange: setRowSelection,
  onGlobalFilterChange: setGlobalFilter,
  getCoreRowModel: getCoreRowModel(),
  getSortedRowModel: getSortedRowModel(),
  getFilteredRowModel: getFilteredRowModel(),
  getGroupedRowModel: getGroupedRowModel(),
  getExpandedRowModel: getExpandedRowModel(),
  getFacetedRowModel: getFacetedRowModel(),
  getFacetedUniqueValues: getFacetedUniqueValues(),
  enableRowSelection: true,
  enableMultiRowSelection: true,
  // Do NOT use getPaginationRowModel with virtualization
});
```

### Virtual scrolling integration

The critical architectural pattern uses `display: grid` on the `<table>`, `position: sticky` on the `<thead>`, and `position: absolute` with `translateY` on each `<tr>`. **For React 19, set `useFlushSync: false`** to prevent lifecycle method warnings:

```tsx
import { useVirtualizer } from '@tanstack/react-virtual';

const tableContainerRef = useRef<HTMLDivElement>(null);
const { rows } = table.getRowModel();

const rowVirtualizer = useVirtualizer<HTMLDivElement, HTMLTableRowElement>({
  count: rows.length,
  getScrollElement: () => tableContainerRef.current,
  estimateSize: () => 40,
  overscan: 10,
  useFlushSync: false, // Critical for React 19
});

return (
  <div ref={tableContainerRef}
    className="overflow-auto relative rounded-lg border border-border"
    style={{ height: '800px' }}>
    <table style={{ display: 'grid' }}>
      <thead style={{ display: 'grid', position: 'sticky', top: 0, zIndex: 1 }}
        className="bg-surface-alt">
        {table.getHeaderGroups().map((headerGroup) => (
          <tr key={headerGroup.id} style={{ display: 'flex', width: '100%' }}>
            {headerGroup.headers.map((header) => (
              <th key={header.id}
                style={{ display: 'flex', width: header.getSize() }}
                className="px-4 py-3 text-left text-xs font-medium text-text-secondary uppercase tracking-wide">
                <div className={header.column.getCanSort() ? 'cursor-pointer select-none' : ''}
                  onClick={header.column.getToggleSortingHandler()}>
                  {flexRender(header.column.columnDef.header, header.getContext())}
                  {{ asc: ' ↑', desc: ' ↓' }[header.column.getIsSorted() as string] ?? null}
                </div>
              </th>
            ))}
          </tr>
        ))}
      </thead>
      <tbody style={{
        display: 'grid',
        height: `${rowVirtualizer.getTotalSize()}px`,
        position: 'relative',
      }}>
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const row = rows[virtualRow.index];
          return (
            <tr key={row.id}
              data-index={virtualRow.index}
              ref={(node) => rowVirtualizer.measureElement(node)}
              style={{
                display: 'flex',
                position: 'absolute',
                transform: `translateY(${virtualRow.start}px)`,
                width: '100%',
              }}
              className={`border-b border-border hover:bg-surface-alt
                ${row.getIsSelected() ? 'bg-primary-50' : ''}`}>
              {row.getVisibleCells().map((cell) => (
                <td key={cell.id}
                  style={{ display: 'flex', width: cell.column.getSize() }}
                  className="px-4 py-2 text-sm text-text">
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </td>
              ))}
            </tr>
          );
        })}
      </tbody>
    </table>
  </div>
);
```

### Performance optimization checklist

- **Stable data reference**: `data` must be `useState` or `useMemo` — a new array reference every render triggers full recalculation
- **Memoize columns**: wrap in `useMemo(() => [...], [])` — column defs should never recreate
- **Accurate `estimateSize`**: match actual row height (40px default) for smooth scrollbar behavior
- **`overscan: 10`**: renders 10 extra rows above/below viewport — smooth scroll without excess DOM
- **Avoid pagination with virtualization**: they are alternative strategies, never complementary
- **For 100K+ rows**: use `manualSorting: true` and `manualFiltering: true` to offload processing to the Rust backend via Tauri commands
- **Accessibility**: add `aria-rowcount={data.length}` to `<table>` since screen readers cannot determine total rows from the virtualized DOM
- **Dynamic positioning via `style`**: Tailwind CSS cannot interpolate JS variables — virtual row positioning (`translateY`, `height`) must use inline `style` attributes

---

## 3. Date range picker with French locale and presets

**react-day-picker v9.14.0** (released February 26, 2026) provides native French locale support with pre-translated labels since v9.12.0. It bundles `date-fns` as a dependency and is compatible with React 16.8+, including React 19.

### Core setup with French locale

```tsx
import { useState } from 'react';
import { DayPicker, getDefaultClassNames } from 'react-day-picker';
import { fr } from 'react-day-picker/locale';
import type { DateRange } from 'react-day-picker';
import 'react-day-picker/style.css';
```

The `DateRange` type is `{ from: Date | undefined; to?: Date | undefined }`. Import the locale from `react-day-picker/locale` (not from `date-fns/locale/fr`) — the DayPicker wrapper includes pre-translated ARIA labels for all UI elements.

### Presets with sidebar layout

```tsx
import { startOfDay, endOfDay, subDays, startOfQuarter, subQuarters } from 'date-fns';

interface DatePreset {
  label: string;
  range: () => DateRange;
}

const PRESETS: DatePreset[] = [
  {
    label: "Aujourd'hui",
    range: () => ({ from: startOfDay(new Date()), to: endOfDay(new Date()) }),
  },
  {
    label: '7 derniers jours',
    range: () => ({ from: startOfDay(subDays(new Date(), 6)), to: endOfDay(new Date()) }),
  },
  {
    label: '30 derniers jours',
    range: () => ({ from: startOfDay(subDays(new Date(), 29)), to: endOfDay(new Date()) }),
  },
  {
    label: 'Dernier trimestre',
    range: () => {
      const lastQ = subQuarters(new Date(), 1);
      return {
        from: startOfQuarter(lastQ),
        to: endOfDay(subDays(startOfQuarter(new Date()), 1)),
      };
    },
  },
];

export function DateRangeWithPresets({
  value, onChange,
}: { value: DateRange | undefined; onChange: (range: DateRange | undefined) => void }) {
  const [activePreset, setActivePreset] = useState<string | null>(null);
  const defaults = getDefaultClassNames();

  const handlePreset = (preset: DatePreset) => {
    onChange(preset.range());
    setActivePreset(preset.label);
  };

  return (
    <div className="flex gap-4 rounded-xl border border-border bg-surface p-4 shadow-md">
      <div className="flex flex-col gap-1 border-r border-border pr-4 min-w-40">
        {PRESETS.map((p) => (
          <button key={p.label} onClick={() => handlePreset(p)}
            className={`px-3 py-2 text-sm rounded-md text-left transition-colors
              ${activePreset === p.label
                ? 'bg-primary-500 text-white' : 'hover:bg-surface-alt text-text-secondary'}`}>
            {p.label}
          </button>
        ))}
        <button onClick={() => setActivePreset(null)}
          className={`px-3 py-2 text-sm rounded-md text-left transition-colors
            ${!activePreset ? 'bg-primary-500 text-white' : 'hover:bg-surface-alt text-text-secondary'}`}>
          Plage personnalisée
        </button>
      </div>
      <DayPicker
        locale={fr}
        mode="range"
        numberOfMonths={2}
        selected={value}
        onSelect={(range) => { onChange(range); setActivePreset(null); }}
        classNames={{
          root: `${defaults.root}`,
          today: 'border-primary-400',
          selected: 'bg-primary-500 text-white',
          range_start: 'bg-primary-600 text-white rounded-l-full',
          range_end: 'bg-primary-600 text-white rounded-r-full',
          range_middle: 'bg-primary-100 text-primary-800',
          chevron: `${defaults.chevron} fill-primary-500`,
        }}
      />
    </div>
  );
}
```

### Tailwind CSS 4 styling via CSS variables

Rather than overriding each `classNames` key, apply global CSS variable overrides to align with the CPAM theme:

```css
.rdp-root {
  --rdp-accent-color: var(--color-primary-500);
  --rdp-accent-background-color: var(--color-primary-50);
  --rdp-range_middle-background-color: var(--color-primary-100);
  --rdp-range_middle-color: var(--color-primary-800);
  --rdp-today-color: var(--color-primary-400);
}
```

### Integration with temporal KPIs

The date range state feeds into all dashboard components — charts, KPIs, and the ticket table. A shared context or Zustand store holds the active range:

```typescript
// In the dashboard's filter store
const useDashboardFilters = create<{
  dateRange: DateRange | undefined;
  setDateRange: (range: DateRange | undefined) => void;
}>((set) => ({
  dateRange: undefined,
  setDateRange: (range) => set({ dateRange: range }),
}));

// All chart/table components read from this store
// and filter their data accordingly through the Rust backend
// via Tauri invoke: invoke('filter_tickets', { from: range?.from, to: range?.to })
```

---

## 4. Word cloud powered by @visx/wordcloud

**@visx/wordcloud v3.12.0** (~14.3 KB) is the clear choice. Part of the Airbnb-maintained visx ecosystem with full TypeScript support and React 19 compatibility, it uses a render prop pattern that gives complete control over SVG rendering, click handlers, and styling — at a fraction of the bundle size of alternatives. The abandoned `react-wordcloud` (last release 6 years ago) is incompatible with React 19. A modernized fork `@cp949/react-wordcloud` (v1.0.1, 120 KB) exists as a batteries-included alternative but has a much larger bundle.

### Integration with TF-IDF data from Rust backend

The Rust backend (Segment 5) outputs `Vec<(String, f64)>` representing word–TF-IDF weight pairs. The word cloud maps these directly:

```tsx
import { Wordcloud } from '@visx/wordcloud';
import { Text } from '@visx/text';
import { scaleLog } from '@visx/scale';
import { useMemo, useState, useCallback } from 'react';

interface TfIdfWord {
  text: string;
  value: number;
}

interface WordCloudProps {
  words: TfIdfWord[];
  width: number;
  height: number;
  onWordClick: (word: string) => void; // Filter tickets by clicked word
}

const CLOUD_COLORS = ['#0C419A', '#0072B2', '#009E73', '#D55E00', '#56B4E9', '#CC79A7', '#228833', '#E69F00'];

export function TicketWordCloud({ words, width, height, onWordClick }: WordCloudProps) {
  const [hoveredWord, setHoveredWord] = useState<string | null>(null);

  const fontScale = useMemo(() => {
    const values = words.map((w) => w.value);
    return scaleLog({
      domain: [Math.min(...values), Math.max(...values)],
      range: [14, 72],
    });
  }, [words]);

  const handleClick = useCallback((w: TfIdfWord) => {
    onWordClick(w.text);
  }, [onWordClick]);

  return (
    <svg width={width} height={height}>
      <Wordcloud
        words={words}
        width={width}
        height={height}
        fontSize={(d) => fontScale(d.value)}
        font="Inter, sans-serif"
        padding={3}
        spiral="archimedean"
        rotate={0}               // Horizontal only for French readability
        random={() => 0.5}       // Deterministic layout
      >
        {(cloudWords) =>
          cloudWords.map((w, i) => (
            <Text
              key={w.text}
              fill={CLOUD_COLORS[i % CLOUD_COLORS.length]}
              textAnchor="middle"
              transform={`translate(${w.x}, ${w.y}) rotate(${w.rotate})`}
              fontSize={w.size}
              fontFamily={w.font}
              fontWeight={hoveredWord === w.text ? 700 : 500}
              opacity={hoveredWord && hoveredWord !== w.text ? 0.4 : 1}
              onClick={() => handleClick({ text: w.text!, value: (w as any).value })}
              onMouseEnter={() => setHoveredWord(w.text!)}
              onMouseLeave={() => setHoveredWord(null)}
              style={{ cursor: 'pointer', transition: 'opacity 0.2s, font-weight 0.2s' }}
            >
              {w.text}
            </Text>
          ))
        }
      </Wordcloud>
    </svg>
  );
}
```

French accented characters (é, è, ê, ç, à, ù, etc.) render natively in SVG `<text>` elements with no special handling required. Use `rotate={0}` (horizontal-only layout) for optimal French text legibility. The `scaleLog` mapping ensures TF-IDF weights produce visually proportional font sizes, with logarithmic scaling preventing high-weight outliers from dominating.

---

## 5. ECharts drill-down patterns for hierarchical GLPI data

The CPAM GLPI data has three natural hierarchies that benefit from drill-down: **category levels** (Cat1 → Cat2 → Cat3), **organizational groups** (_DSI → _SUPPORT → _PARC), and **technician → keywords** thematic analysis. ECharts provides built-in drill-down for both treemap and sunburst chart types.

### Treemap drill-down with breadcrumbs

The `leafDepth` property is the key control. Setting `leafDepth: 1` shows only one level at a time — clicking a node zooms into its children. The built-in breadcrumb bar shows the navigation path and allows clicking back to any ancestor:

```typescript
const treemapOption: ECOption = {
  tooltip: { formatter: '{b}: {c} tickets' },
  series: [{
    type: 'treemap',
    leafDepth: 1,                          // Show one level at a time
    nodeClick: 'zoomToNode',               // 'zoomToNode' | 'link' | false
    roam: true,
    drillDownIcon: '▶',
    animation: true,
    animationDurationUpdate: 900,
    animationEasing: 'quinticInOut',
    breadcrumb: {
      show: true,
      left: 'center',
      top: 'bottom',
      height: 28,
      itemStyle: {
        color: 'var(--color-primary-500)',
        textStyle: { color: '#ffffff', fontSize: 13 },
      },
      emphasis: { itemStyle: { color: 'var(--color-primary-600)' } },
    },
    levels: [
      { itemStyle: { borderColor: 'var(--color-border)', borderWidth: 2, gapWidth: 2 } },
      { colorSaturation: [0.35, 0.5], itemStyle: { borderColorSaturation: 0.6, gapWidth: 1 } },
      { colorSaturation: [0.35, 0.5], itemStyle: { borderColorSaturation: 0.7, gapWidth: 1 } },
    ],
    data: categoryHierarchy, // [{ name: 'Cat1', children: [{ name: 'Cat2', children: [...] }] }]
  }],
};
```

### Sunburst drill-down for group hierarchy

Sunburst drill-down is the default behavior — clicking a sector drills in, and clicking the center circle returns to the parent. The `nodeClick: 'rootToNode'` setting (default) handles everything:

```typescript
const sunburstOption: ECOption = {
  series: [{
    type: 'sunburst',
    nodeClick: 'rootToNode',
    radius: ['0%', '90%'],
    sort: null,
    emphasis: { focus: 'ancestor' },
    levels: [
      { /* Level 0: center circle (back button after drill-down) */ },
      { r0: '15%', r: '40%', label: { rotate: 'tangential', fontSize: 13 } },
      { r0: '40%', r: '65%', label: { align: 'right', fontSize: 12 } },
      { r0: '65%', r: '85%', label: { position: 'outside', padding: 3 } },
    ],
    data: groupHierarchy,
    // e.g. [{ name: '_DSI', children: [{ name: '_SUPPORT', children: [{ name: '_PARC', value: 42 }] }] }]
  }],
};
```

### React state management for custom drill navigation

When you need more control than the built-in drill-down (custom breadcrumbs, external back button, syncing drill level with other UI), manage drill state explicitly:

```typescript
interface DrillState {
  path: Array<{ name: string; dataIndex: number }>;
  currentData: TreeNode[];
}

const [drillState, setDrillState] = useState<DrillState>({
  path: [],
  currentData: fullHierarchy,
});

const onEvents = useMemo(() => ({
  click: (params: any) => {
    if (params.data?.children?.length) {
      setDrillState((prev) => ({
        path: [...prev.path, { name: params.name, dataIndex: params.dataIndex }],
        currentData: params.data.children,
      }));
    }
  },
}), []);

// Breadcrumb click handler — navigate back to any level
const drillTo = useCallback((levelIndex: number) => {
  const newPath = drillState.path.slice(0, levelIndex);
  let node: any = { children: fullHierarchy };
  for (const step of newPath) {
    node = node.children.find((c: any) => c.name === step.name);
  }
  setDrillState({ path: newPath, currentData: node.children ?? fullHierarchy });
}, [drillState.path, fullHierarchy]);
```

### Custom breadcrumb component

```tsx
function DrillBreadcrumb({ path, onNavigate }: {
  path: Array<{ name: string }>;
  onNavigate: (level: number) => void;
}) {
  return (
    <nav className="flex items-center gap-1 text-sm text-text-secondary mb-2">
      <button onClick={() => onNavigate(0)}
        className="hover:text-primary-500 font-medium">Racine</button>
      {path.map((step, i) => (
        <span key={i} className="flex items-center gap-1">
          <span className="text-text-muted">/</span>
          <button onClick={() => onNavigate(i + 1)}
            className="hover:text-primary-500 font-medium">{step.name}</button>
        </span>
      ))}
    </nav>
  );
}
```

### Animated transitions with Universal Transition

ECharts' `UniversalTransition` feature (imported above) enables morphing animations when switching data between drill levels. Use `series.dataGroupId` and `data[].groupId` to establish parent-child relationships:

```typescript
// Parent data
[{ name: 'Category A', value: 100, groupId: 'catA' }]

// After drilling into Category A, children reference parent via groupId
series: [{
  type: 'treemap',
  dataGroupId: 'catA',  // Links this series to the parent item
  universalTransition: { enabled: true, divideShape: 'split' },
  data: categoryAChildren,
}]
```

This produces smooth split/merge animations as nodes expand into children or collapse back to parents.

---

## 6. KPI card components with thresholds and sparklines

The CPAM dashboard displays key metrics (stock total, âge moyen, tickets > 90 jours, tickets par technicien) as color-coded KPI cards with trend indicators and embedded sparklines.

### Component architecture

```typescript
interface KpiCardProps {
  title: string;
  value: number;
  previousValue?: number;
  format?: 'number' | 'days' | 'percent';
  sparklineData?: number[];
  thresholds?: { green: number; yellow: number; orange: number; red: number };
  icon?: React.ReactNode;
}

type ThresholdLevel = 'green' | 'yellow' | 'orange' | 'red';

function getThresholdLevel(value: number, t: KpiCardProps['thresholds']): ThresholdLevel {
  if (!t) return 'green';
  if (value <= t.green) return 'green';
  if (value <= t.yellow) return 'yellow';
  if (value <= t.orange) return 'orange';
  return 'red';
}

const THRESHOLD_STYLES: Record<ThresholdLevel, { border: string; text: string; bg: string }> = {
  green:  { border: 'border-l-success-500',  text: 'text-success-600',  bg: 'bg-success-50' },
  yellow: { border: 'border-l-warning-500',  text: 'text-warning-600',  bg: 'bg-warning-50' },
  orange: { border: 'border-l-[#d4600a]',    text: 'text-[#d4600a]',   bg: 'bg-orange-50' },
  red:    { border: 'border-l-danger-500',    text: 'text-danger-600',  bg: 'bg-danger-50' },
};
```

### Trend indicator

```tsx
function TrendIndicator({ current, previous }: { current: number; previous?: number }) {
  if (previous === undefined) return null;
  const delta = ((current - previous) / previous) * 100;
  const direction = delta > 0 ? 'up' : delta < 0 ? 'down' : 'flat';
  const colorClass = direction === 'up' ? 'text-danger-600 bg-danger-50'   // Up is bad for ticket counts
                   : direction === 'down' ? 'text-success-600 bg-success-50'
                   : 'text-text-muted bg-surface-alt';
  return (
    <span className={`inline-flex items-center gap-0.5 rounded-full px-2 py-0.5 text-xs font-medium ${colorClass}`}>
      {direction === 'up' ? '↑' : direction === 'down' ? '↓' : '→'}
      {Math.abs(delta).toFixed(1)}%
    </span>
  );
}
```

### Custom SVG sparkline (zero dependencies)

Rather than adding react-sparklines or another library, a 15-line SVG component keeps the bundle minimal while ECharts handles complex charts:

```tsx
function Sparkline({ data, width = 120, height = 32, color = 'var(--color-primary-500)' }:
  { data: number[]; width?: number; height?: number; color?: string }) {
  if (data.length < 2) return null;
  const max = Math.max(...data);
  const min = Math.min(...data);
  const range = max - min || 1;
  const points = data.map((v, i) =>
    `${(i / (data.length - 1)) * width},${height - ((v - min) / range) * (height - 4) - 2}`
  ).join(' ');
  const areaPoints = `${points} ${width},${height} 0,${height}`;
  return (
    <svg width={width} height={height} className="overflow-visible">
      <polyline fill="none" stroke={color} strokeWidth="1.5" strokeLinecap="round"
        strokeLinejoin="round" points={points} />
      <polygon fill={color} opacity="0.1" points={areaPoints} />
    </svg>
  );
}
```

### Complete KPI card

```tsx
export function KpiCard({ title, value, previousValue, format = 'number', sparklineData, thresholds, icon }: KpiCardProps) {
  const level = getThresholdLevel(value, thresholds);
  const styles = THRESHOLD_STYLES[level];

  const formattedValue = format === 'days' ? `${value} j`
    : format === 'percent' ? `${value.toFixed(1)}%`
    : value.toLocaleString('fr-FR');

  return (
    <div className={`rounded-lg border border-border bg-surface p-5 shadow-xs
      border-l-4 ${styles.border} transition-shadow hover:shadow-sm`}>
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium text-text-secondary">{title}</p>
        {icon && <span className="text-text-muted">{icon}</span>}
      </div>
      <div className="mt-2 flex items-baseline justify-between">
        <span className={`text-3xl font-semibold ${styles.text}`}>{formattedValue}</span>
        <TrendIndicator current={value} previous={previousValue} />
      </div>
      {sparklineData && (
        <div className="mt-3">
          <Sparkline data={sparklineData} color={level === 'green' ? 'var(--color-success-500)' : 'var(--color-danger-500)'} />
        </div>
      )}
    </div>
  );
}
```

### Responsive dashboard grid

```tsx
<div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
  <KpiCard title="Stock total" value={847} previousValue={912}
    thresholds={{ green: 500, yellow: 700, orange: 900, red: 1000 }}
    sparklineData={stockHistory} />
  <KpiCard title="Âge moyen" value={23} previousValue={19} format="days"
    thresholds={{ green: 10, yellow: 20, orange: 40, red: 60 }}
    sparklineData={ageHistory} />
  <KpiCard title="Tickets > 90 jours" value={34} previousValue={28}
    thresholds={{ green: 10, yellow: 20, orange: 40, red: 50 }}
    sparklineData={over90History} />
  <KpiCard title="Taux résolution" value={78.4} previousValue={82.1} format="percent"
    thresholds={{ green: 90, yellow: 80, orange: 70, red: 60 }} />
</div>
```

Note the **inverted threshold logic** for resolution rate — higher is better. The `getThresholdLevel` function can be parameterized with a `higherIsBetter` flag for such metrics.

---

## 7. Professional CPAM light theme with Tailwind CSS 4

Tailwind CSS 4 replaces `tailwind.config.js` with **CSS-first configuration** using the `@theme` directive. Every design token defined in `@theme` generates corresponding utility classes. The palette below is built from the **Ameli/CPAM brand blue (#0C419A)**, aligned with the French institutional identity (DSFR Blue France #000091 as reference), and validated for **WCAG AA contrast compliance**.

### Complete theme file

```css
@import "tailwindcss";

/* Class-based dark mode (for future use) */
@custom-variant dark (&:where(.dark, .dark *));

@theme {
  /* ── Primary: CPAM Institutional Blue ── */
  --color-primary-50:  #f0f4fa;
  --color-primary-100: #dce5f5;
  --color-primary-200: #b8cbeb;
  --color-primary-300: #8baade;
  --color-primary-400: #5e89d1;
  --color-primary-500: #0C419A;   /* Brand blue — 7.2:1 on white ✓ AAA */
  --color-primary-600: #0a3783;
  --color-primary-700: #082d6c;
  --color-primary-800: #062355;
  --color-primary-900: #04193e;
  --color-primary-950: #020e24;

  /* ── Secondary: Teal accent ── */
  --color-secondary-50:  #f0fafb;
  --color-secondary-100: #d0f0f5;
  --color-secondary-200: #a1e0eb;
  --color-secondary-300: #5ec8d8;
  --color-secondary-400: #2dafc4;
  --color-secondary-500: #1a8fa3;
  --color-secondary-600: #157485;
  --color-secondary-700: #115c6a;
  --color-secondary-800: #0d4550;
  --color-secondary-900: #092f37;

  /* ── Neutral: Blue-tinted grays ── */
  --color-gray-50:  #f8f9fb;
  --color-gray-100: #f1f3f7;
  --color-gray-200: #e2e6ee;
  --color-gray-300: #cdd3df;
  --color-gray-400: #9da5b8;
  --color-gray-500: #6e7891;
  --color-gray-600: #525d73;
  --color-gray-700: #3d4559;
  --color-gray-800: #2a3040;
  --color-gray-900: #1a1f2e;
  --color-gray-950: #0f1219;

  /* ── Semantic colors ── */
  --color-success-50:  #f0faf4;
  --color-success-500: #18753c;    /* 5.5:1 on white ✓ AA */
  --color-success-600: #136130;

  --color-warning-50:  #fef9ec;
  --color-warning-500: #b27806;    /* 4.6:1 on white ✓ AA */
  --color-warning-600: #965e04;

  --color-danger-50:  #fef2f2;
  --color-danger-500: #ce0500;     /* 5.2:1 on white ✓ AA */
  --color-danger-600: #af0400;

  --color-info-500: #2563eb;
  --color-info-600: #1d4ed8;

  /* ── Surface and semantic aliases ── */
  --color-background:      #f8f9fb;
  --color-surface:         #ffffff;
  --color-surface-alt:     #f1f3f7;
  --color-border:          #e2e6ee;
  --color-border-strong:   #cdd3df;
  --color-text:            #1a1f2e;  /* 15.6:1 on white ✓ AAA */
  --color-text-secondary:  #525d73;  /* 5.8:1 on white ✓ AA */
  --color-text-muted:      #6e7891;  /* 4.1:1 — large text only */

  /* ── Threshold indicators ── */
  --color-threshold-green:  #18753c;
  --color-threshold-yellow: #b27806;
  --color-threshold-orange: #d4600a;
  --color-threshold-red:    #ce0500;

  /* ── Typography ── */
  --font-sans: "Inter", "Segoe UI", system-ui, -apple-system, sans-serif;
  --font-mono: "JetBrains Mono", "Fira Code", ui-monospace, monospace;

  /* Desktop-optimized sizes (denser than web defaults) */
  --text-xs:   0.6875rem;  /* 11px */
  --text-sm:   0.8125rem;  /* 13px */
  --text-base: 0.875rem;   /* 14px — default for data-dense desktop UI */
  --text-lg:   1rem;       /* 16px */
  --text-xl:   1.125rem;   /* 18px */
  --text-2xl:  1.375rem;   /* 22px */
  --text-3xl:  1.75rem;    /* 28px */
  --text-4xl:  2.25rem;    /* 36px — KPI hero values */

  /* ── Spacing (4px base unit) ── */
  --spacing: 0.25rem;

  /* ── Border radius ── */
  --radius-sm:  0.25rem;
  --radius-md:  0.375rem;
  --radius-lg:  0.5rem;
  --radius-xl:  0.75rem;

  /* ── Shadows (subtle, professional) ── */
  --shadow-xs: 0 1px 2px 0 rgb(26 31 46 / 0.04);
  --shadow-sm: 0 1px 3px 0 rgb(26 31 46 / 0.06), 0 1px 2px -1px rgb(26 31 46 / 0.06);
  --shadow-md: 0 4px 6px -1px rgb(26 31 46 / 0.06), 0 2px 4px -2px rgb(26 31 46 / 0.06);
  --shadow-lg: 0 10px 15px -3px rgb(26 31 46 / 0.06), 0 4px 6px -4px rgb(26 31 46 / 0.06);
}

@layer base {
  html {
    font-size: 16px;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
  body {
    background-color: var(--color-background);
    color: var(--color-text);
    font-family: var(--font-sans);
    font-size: var(--text-base);
    line-height: 1.5;
  }
}
```

### Colorblind-friendly chart palette

The chart palette combines the **Okabe-Ito palette** (Wong, 2011) and **Paul Tol's Bright scheme** — both validated for protanopia, deuteranopia, and tritanopia. The CPAM brand blue anchors position 0:

```typescript
// Shared across ECharts, Nivo, and @visx/wordcloud
export const CHART_PALETTE = [
  '#0C419A',  // CPAM Blue (brand anchor)
  '#E69F00',  // Amber
  '#009E73',  // Teal Green
  '#D55E00',  // Vermillion
  '#56B4E9',  // Sky Blue
  '#CC79A7',  // Rose Pink
  '#0072B2',  // Deep Blue
  '#228833',  // Forest Green
  '#EE6677',  // Salmon
  '#AA3377',  // Purple
  '#4477AA',  // Steel Blue
  '#F0E442',  // Yellow (use sparingly on white)
] as const;

// Sequential heatmap gradient: green → yellow → orange → red
export const HEATMAP_GRADIENT = ['#18753c', '#6bab3e', '#b27806', '#d4600a', '#ce0500'];
```

### Registering a unified ECharts theme

```typescript
import * as echarts from 'echarts/core';

echarts.registerTheme('cpam', {
  color: [...CHART_PALETTE],
  backgroundColor: 'transparent',
  textStyle: { fontFamily: 'Inter, Segoe UI, system-ui, sans-serif', fontSize: 13, color: '#1a1f2e' },
  title: {
    textStyle: { fontSize: 16, fontWeight: 600, color: '#1a1f2e' },
    subtextStyle: { fontSize: 13, color: '#525d73' },
  },
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

// Initialize any chart with: echarts.init(container, 'cpam')
```

The same palette exported as a Nivo theme object ensures visual consistency when mixing ECharts (treemap, sunburst, heatmap, Sankey) with Nivo or visx (word cloud):

```typescript
export const nivoTheme = {
  text: { fontSize: 13, fill: '#1a1f2e', fontFamily: 'Inter, sans-serif' },
  axis: {
    domain: { line: { stroke: '#cdd3df' } },
    ticks: { text: { fill: '#525d73', fontSize: 12 } },
  },
  grid: { line: { stroke: '#f1f3f7', strokeDasharray: '4 4' } },
  tooltip: {
    container: {
      background: '#fff', color: '#1a1f2e', fontSize: 13,
      borderRadius: '6px', boxShadow: '0 4px 6px -1px rgb(26 31 46 / 0.06)',
      border: '1px solid #e2e6ee',
    },
  },
};
```

---

## Conclusion: complete technology stack for Segment 7

The frontend stack for the CPAM GLPI Dashboard consolidates around a tight set of libraries, each chosen for a specific strength. **Apache ECharts 6** handles all four advanced chart types with native drill-down, Canvas performance, and a unified `cpam` theme — no other library matches its combination of treemap breadcrumbs, sunburst click-to-drill, heatmap VisualMap, and Sankey draggable nodes. **TanStack Table v8.21.3** paired with **react-virtual v3.13.19** delivers a fully virtualized, type-safe ticket grid that maintains 60 FPS across 50,000+ rows with `useFlushSync: false` for React 19 compatibility. **react-day-picker v9.14.0** provides French locale out of the box since v9.12.0. **@visx/wordcloud** at just 14.3 KB gives maximum control over TF-IDF word rendering with native click-to-filter. The Tailwind CSS 4 `@theme` directive replaces `tailwind.config.js` entirely, defining all design tokens — including the CPAM brand blue (#0C419A), WCAG AA–compliant semantic colors, and the Okabe-Ito colorblind-friendly chart palette — in a single CSS file that generates every utility class the application needs.

|Component|Library|Version|Bundle (gzip)|
|---|---|---|---|
|Charts (treemap, sunburst, heatmap, Sankey)|Apache ECharts + custom wrapper|6.0.0|~100–150 KB (tree-shaken)|
|Data table|@tanstack/react-table|8.21.3|~15 KB|
|Virtual scrolling|@tanstack/react-virtual|3.13.19|~3.9 KB|
|Date range picker|react-day-picker|9.14.0|~20 KB|
|Word cloud|@visx/wordcloud|3.12.0|~14.3 KB|
|Styling|Tailwind CSS|4.2|— (build-time)|

The custom ECharts wrapper (~50 lines) eliminates the `echarts-for-react` dependency while providing `useMemo`-safe event handling and `ResizeObserver`-based responsive resizing. All libraries are validated for React 19 without requiring the React Compiler, and none depend on SSR infrastructure — ideal for the Tauri desktop context.