# Frontend Theme — GLPI Dashboard CPAM

Source : Segment 7

---

## Stack Frontend

| Composant | Bibliothèque | Version | Bundle (gzip) |
|---|---|---|---|
| Charts (treemap, sunburst, heatmap, Sankey) | Apache ECharts + custom wrapper | 6.0.0 | ~100–150 KB (tree-shaken) |
| Data table | @tanstack/react-table | 8.21.3 | ~15 KB |
| Virtual scrolling | @tanstack/react-virtual | 3.13.19 | ~3.9 KB |
| Date range picker | react-day-picker | 9.14.0 | ~20 KB |
| Word cloud | @visx/wordcloud | 3.12.0 | ~14.3 KB |
| Styling | Tailwind CSS | 4.2 | — (build-time) |

**Note React 19** : `useFlushSync: false` sur `useVirtualizer` — critique pour éviter les warnings lifecycle.

---

## Thème Tailwind CSS 4 (CSS-first avec `@theme`)

Tailwind CSS 4 remplace `tailwind.config.js` par la directive `@theme` dans le CSS.

```css
@import "tailwindcss";

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
  --color-secondary-500: #1a8fa3;
  --color-secondary-600: #157485;

  /* ── Neutral: Blue-tinted grays ── */
  --color-gray-50:  #f8f9fb;
  --color-gray-100: #f1f3f7;
  --color-gray-200: #e2e6ee;
  --color-gray-300: #cdd3df;
  --color-gray-500: #6e7891;
  --color-gray-700: #3d4559;
  --color-gray-900: #1a1f2e;

  /* ── Semantic colors (WCAG AA compliant) ── */
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

---

## Palette Charts (Okabe-Ito + Paul Tol Bright)

Validée protanopie, deutéranopie, tritanopie. CPAM Blue en position 0.

```typescript
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

// Gradient heatmap : vert → jaune → orange → rouge
export const HEATMAP_GRADIENT = ['#18753c', '#6bab3e', '#b27806', '#d4600a', '#ce0500'];
```

---

## Thème ECharts `cpam`

```typescript
echarts.registerTheme('cpam', {
  color: [...CHART_PALETTE],
  backgroundColor: 'transparent',
  textStyle: {
    fontFamily: 'Inter, Segoe UI, system-ui, sans-serif',
    fontSize: 13,
    color: '#1a1f2e',
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
// Usage : echarts.init(container, 'cpam')
```

---

## Seuils Couleurs KPI Cards (Tailwind)

```typescript
const THRESHOLD_STYLES = {
  green:  { border: 'border-l-success-500',  text: 'text-success-600',  bg: 'bg-success-50' },
  yellow: { border: 'border-l-warning-500',  text: 'text-warning-600',  bg: 'bg-warning-50' },
  orange: { border: 'border-l-[#d4600a]',    text: 'text-[#d4600a]',   bg: 'bg-orange-50' },
  red:    { border: 'border-l-danger-500',    text: 'text-danger-600',  bg: 'bg-danger-50' },
};
```

---

## Notes Architecture Frontend

- **ECharts** : winner pour treemap (drill-down natif), sunburst, heatmap (VisualMap), Sankey
- **TanStack Table v8.21.3** : prod stable pour React 19 (v9 alpha instable)
- **react-day-picker v9.14.0** : locale `fr` native depuis v9.12.0 (import depuis `react-day-picker/locale`)
- **@visx/wordcloud v3.12.0** : ~14.3 KB, render prop pattern, click-to-filter
- **Custom ECharts wrapper** : ~50 lignes, ResizeObserver, élimine `echarts-for-react`
- **Virtual scrolling** : `display: grid` sur `<table>`, `position: absolute` + `translateY` sur `<tr>`
- **Positioning dynamique** : utiliser `style={{}}` inline (pas Tailwind) pour `translateY`, `height` virtualisés
