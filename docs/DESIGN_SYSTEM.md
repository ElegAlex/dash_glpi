# Design System — PILOTAGE GLPI · Material Design Épuré

> **Version** 1.0 · Mars 2026
> **Stack** Tailwind CSS 4 + React 19 + ECharts 6 + Tauri 2.10+
> **Résumé des règles** : voir `CLAUDE.md` section "Design System"

---

## Table des matières

1. [Philosophie](#1-philosophie)
2. [Palette de couleurs](#2-palette-de-couleurs)
3. [Système d'ombres Material](#3-système-dombres-material)
4. [Typographie](#4-typographie)
5. [Composants — Code Tailwind](#5-composants--code-tailwind)
6. [Thème ECharts Material](#6-thème-echarts-material)
7. [Layout de page](#7-layout-de-page)
8. [Spacing & border-radius](#8-spacing--border-radius)
9. [Transitions & animations](#9-transitions--animations)
10. [Checklist d'implémentation](#10-checklist-dimplémentation)

---

## 1. Philosophie

Trois principes directeurs :

**Élévation par les ombres** — Les surfaces se distinguent par leur profondeur, pas par des bordures. Les cards flottent au-dessus du fond. On n'utilise JAMAIS `border-gray-*` comme séparateur de blocs principaux.

**Couleurs vivantes et intentionnelles** — Le bleu CPAM (#0C419A) reste la couleur primaire. L'amber (#FFC107–#FF8F00) sert d'accent pour les CTAs, highlights et la ligne Stock net. Les couleurs sémantiques (vert/rouge) sont réservées aux tendances et seuils.

**Typographie hiérarchique** — Les chiffres clés (KPI héros) sont mis en vedette avec DM Sans 700 en grande taille. Le texte courant utilise Source Sans 3 pour la lisibilité. L'échelle est optimisée desktop dense (14px base).

---

## 2. Palette de couleurs

### 2.1. Thème Tailwind CSS 4 complet

Remplacer intégralement le `@theme` dans `src/app.css` :

```css
@import "tailwindcss";

@custom-variant dark (&:where(.dark, .dark *));

@theme {
  /* ── Primary: CPAM Blue ── */
  --color-primary-50:  #EEF2FF;
  --color-primary-100: #D8E0FF;
  --color-primary-200: #B0C1FF;
  --color-primary-300: #7A96F5;
  --color-primary-400: #4B6DE0;
  --color-primary-500: #0C419A;    /* Brand — 7.2:1 sur blanc ✓ AAA */
  --color-primary-600: #0A3580;
  --color-primary-700: #082A66;
  --color-primary-800: #061F4D;
  --color-primary-900: #041433;

  /* ── Accent: Amber vif ── */
  --color-accent-50:  #FFF8E1;
  --color-accent-100: #FFECB3;
  --color-accent-200: #FFE082;
  --color-accent-300: #FFD54F;
  --color-accent-400: #FFCA28;
  --color-accent-500: #FFC107;
  --color-accent-600: #FFB300;
  --color-accent-700: #FF8F00;

  /* ── Surfaces ── */
  --color-surface-base:     #F5F7FA;
  --color-surface-card:     #FFFFFF;
  --color-surface-overlay:  rgba(12, 65, 154, 0.04);

  /* ── Neutral: Slate tintés ── */
  --color-gray-50:  #F8FAFC;
  --color-gray-100: #F1F5F9;
  --color-gray-200: #E2E8F0;
  --color-gray-300: #CBD5E1;
  --color-gray-400: #94A3B8;
  --color-gray-500: #64748B;
  --color-gray-600: #475569;
  --color-gray-700: #334155;
  --color-gray-800: #1E293B;
  --color-gray-900: #0F172A;

  /* ── Sémantiques ── */
  --color-success-50:  #E8F5E9;
  --color-success-500: #2E7D32;
  --color-success-600: #1B5E20;

  --color-warning-50:  #FFF3E0;
  --color-warning-500: #E65100;
  --color-warning-600: #BF360C;

  --color-danger-50:  #FFEBEE;
  --color-danger-500: #C62828;
  --color-danger-600: #B71C1C;

  --color-info-500: #1565C0;
  --color-info-600: #0D47A1;

  /* ── Seuils (inchangés, usage Excel + badges) ── */
  --color-threshold-green:  #2E7D32;
  --color-threshold-yellow: #E65100;
  --color-threshold-orange: #D84315;
  --color-threshold-red:    #C62828;

  /* ── Typography ── */
  --font-display: "DM Sans", system-ui, sans-serif;
  --font-body:    "Source Sans 3", "Segoe UI", system-ui, sans-serif;
  --font-mono:    "JetBrains Mono", "Fira Code", ui-monospace, monospace;

  /* Desktop-optimized sizes */
  --text-xs:   0.6875rem;  /* 11px */
  --text-sm:   0.8125rem;  /* 13px */
  --text-base: 0.875rem;   /* 14px */
  --text-lg:   1rem;        /* 16px */
  --text-xl:   1.25rem;     /* 20px */
  --text-2xl:  1.5rem;      /* 24px */
  --text-3xl:  2rem;        /* 32px */
  --text-4xl:  2.75rem;     /* 44px — KPI héros */

  /* ── Spacing ── */
  --spacing: 0.25rem;

  /* ── Border radius ── */
  --radius-sm:  0.25rem;   /* 4px */
  --radius-md:  0.5rem;    /* 8px */
  --radius-lg:  0.75rem;   /* 12px */
  --radius-xl:  1rem;      /* 16px — cards */
  --radius-2xl: 1rem;      /* 16px — alias */

  /* ── Shadows Material (5 niveaux) ── */
  --shadow-1: 0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06);
  --shadow-2: 0 3px 6px rgba(0,0,0,0.10), 0 2px 4px rgba(0,0,0,0.06);
  --shadow-3: 0 10px 20px rgba(0,0,0,0.10), 0 3px 6px rgba(0,0,0,0.06);
  --shadow-4: 0 15px 30px rgba(0,0,0,0.12), 0 5px 15px rgba(0,0,0,0.08);
  --shadow-5: 0 20px 40px rgba(0,0,0,0.14), 0 8px 20px rgba(0,0,0,0.10);
}

@layer base {
  html {
    font-size: 16px;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
  body {
    background-color: var(--color-surface-base);
    color: var(--color-gray-800);
    font-family: var(--font-body);
    font-size: var(--text-base);
    line-height: 1.5;
  }

  /* Scrollbar custom (Tauri/Webkit) */
  ::-webkit-scrollbar { width: 6px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: rgba(148, 163, 184, 0.4); border-radius: 3px; }
  ::-webkit-scrollbar-thumb:hover { background: rgba(100, 116, 139, 0.6); }
}
```

### 2.2. @font-face (app offline, pas de Google Fonts CDN)

Télécharger DM Sans et Source Sans 3 en woff2 dans `public/fonts/`, puis :

```css
@font-face {
  font-family: "DM Sans";
  src: url("/fonts/DMSans-Medium.woff2") format("woff2");
  font-weight: 500;
  font-display: swap;
}
@font-face {
  font-family: "DM Sans";
  src: url("/fonts/DMSans-SemiBold.woff2") format("woff2");
  font-weight: 600;
  font-display: swap;
}
@font-face {
  font-family: "DM Sans";
  src: url("/fonts/DMSans-Bold.woff2") format("woff2");
  font-weight: 700;
  font-display: swap;
}
@font-face {
  font-family: "Source Sans 3";
  src: url("/fonts/SourceSans3-Regular.woff2") format("woff2");
  font-weight: 400;
  font-display: swap;
}
@font-face {
  font-family: "Source Sans 3";
  src: url("/fonts/SourceSans3-SemiBold.woff2") format("woff2");
  font-weight: 600;
  font-display: swap;
}
```

### 2.3. Palette graphiques (Okabe-Ito + Material 800)

Colorblind-safe, validée protanopie/deutéranopie/tritanopie :

| Index | Hex | Nom | Usage principal |
|-------|-----|-----|-----------------|
| 1 | #1565C0 | Blue 800 | Entrants, barres principales |
| 2 | #2E7D32 | Green 800 | Sortants |
| 3 | #FF8F00 | Amber 800 | Stock net, lignes tendance |
| 4 | #6A1B9A | Purple 800 | Catégories tertiaires |
| 5 | #00838F | Cyan 800 | Séries complémentaires |
| 6 | #C62828 | Red 800 | Alertes, anomalies |
| 7 | #4E342E | Brown 800 | Groupes secondaires |
| 8 | #37474F | BlueGrey 800 | Fallback |

---

## 3. Système d'ombres Material

L'élévation est le différenciateur principal par rapport au design actuel. Chaque composant a un niveau de repos et un niveau hover.

| Niveau | Variable | Usage | Valeur CSS |
|--------|----------|-------|------------|
| 0 | — | Éléments plats (lignes de tableau) | `none` |
| 1 | `--shadow-1` | Cards au repos | `0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06)` |
| 2 | `--shadow-2` | Cards hover, boutons | `0 3px 6px rgba(0,0,0,0.10), 0 2px 4px rgba(0,0,0,0.06)` |
| 3 | `--shadow-3` | Dropdowns, popovers, date picker | `0 10px 20px rgba(0,0,0,0.10), 0 3px 6px rgba(0,0,0,0.06)` |
| 4 | `--shadow-4` | Modales, dialogs | `0 15px 30px rgba(0,0,0,0.12), 0 5px 15px rgba(0,0,0,0.08)` |
| 5 | `--shadow-5` | Sidebar (fixe, projetée) | `0 20px 40px rgba(0,0,0,0.14), 0 8px 20px rgba(0,0,0,0.10)` |

### Tableau d'élévation par composant

| Composant | Repos | Hover | Actif |
|-----------|:-----:|:-----:|:-----:|
| Card KPI | 1 | 2 + translateY(-2px) | — |
| Card graphique | 1 | — | — |
| Sidebar | 5 | — | — |
| Date picker popover | 3 | — | — |
| Ligne de tableau | 0 | overlay (primary-500/4%) | — |
| Bouton primaire | 2 | 3 | 1 |
| Tooltip ECharts | 3 | — | — |

---

## 4. Typographie

### Hiérarchie complète

| Élément | Font | Poids | Taille | Couleur | Classe Tailwind |
|---------|------|-------|--------|---------|-----------------|
| Titre de page | DM Sans | 700 | 24px (text-2xl) | slate-800 | `text-2xl font-bold font-[DM_Sans]` |
| Sous-titre page | Source Sans 3 | 400 | 13px (text-sm) | slate-400 | `text-sm text-slate-400` |
| Titre de card | DM Sans | 600 | 16px (text-lg) | slate-700 | `text-lg font-semibold font-[DM_Sans] text-slate-700` |
| KPI héros (grand) | DM Sans | 700 | 44px (text-4xl) | primary-500 ou slate-800 | `text-4xl font-bold font-[DM_Sans] tracking-tight` |
| KPI héros (card) | DM Sans | 700 | 36px (~text-3xl) | slate-800 | `text-[36px] font-bold font-[DM_Sans] tracking-tight` |
| Label KPI | DM Sans | 600 | 11px (text-xs) | slate-400 | `text-xs font-semibold uppercase tracking-wider text-slate-400` |
| Texte courant | Source Sans 3 | 400 | 14px (text-base) | slate-800 | `text-base text-slate-800` |
| Texte secondaire | Source Sans 3 | 400 | 13px (text-sm) | slate-500 | `text-sm text-slate-500` |
| En-tête tableau | DM Sans | 600 | 11px (text-xs) | slate-400 | `text-xs font-semibold uppercase tracking-wider text-slate-400` |
| Cellule tableau | Source Sans 3 | 400 | 14px | slate-600 | `text-sm text-slate-600` |
| Valeur tableau | DM Sans | 600 | 16px | slate-800 | `text-base font-semibold font-[DM_Sans] tabular-nums` |
| Nav sidebar | DM Sans | 500/600 | 14px | white/70 ou white | `text-sm font-medium text-white/70` |

---

## 5. Composants — Code Tailwind

### 5.1. Card (composant de base partagé)

```tsx
// src/components/shared/Card.tsx
interface CardProps {
  children: React.ReactNode;
  className?: string;
  hover?: boolean;
  padding?: 'sm' | 'md' | 'lg';
}

const PADDINGS = { sm: 'p-4', md: 'p-6', lg: 'p-8' } as const;

export function Card({ children, className = '', hover = false, padding = 'md' }: CardProps) {
  return (
    <div className={`
      bg-white rounded-2xl
      shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
      ${hover
        ? 'transition-shadow duration-200 ease-[cubic-bezier(0.4,0,0.2,1)] hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]'
        : ''}
      ${PADDINGS[padding]}
      ${className}
    `}>
      {children}
    </div>
  );
}
```

### 5.2. Sidebar

```tsx
// src/components/layout/Sidebar.tsx
import { Link, useLocation } from 'react-router';
import {
  Upload, BarChart3, FolderTree, TrendingUp,
  Search, GitBranch, Download, Settings,
} from 'lucide-react';

const NAV_ITEMS = [
  { icon: Upload,     label: 'Import',       path: '/import' },
  { icon: BarChart3,  label: 'Stock',        path: '/stock' },
  { icon: FolderTree, label: 'Catégories',   path: '/categories' },
  { icon: TrendingUp, label: 'Bilan',        path: '/bilan' },
  { icon: Search,     label: 'Text Mining',  path: '/textmining' },
  { icon: GitBranch,  label: 'Longitudinal', path: '/longitudinal' },
  { icon: Download,   label: 'Export',        path: '/export' },
  { icon: Settings,   label: 'Paramètres',   path: '/settings' },
] as const;

export function Sidebar() {
  const { pathname } = useLocation();

  return (
    <aside className="
      w-60 min-h-screen flex-shrink-0 flex flex-col z-20
      bg-gradient-to-b from-[#0C419A] to-[#082A66]
      shadow-[0_20px_40px_rgba(0,0,0,0.14),0_8px_20px_rgba(0,0,0,0.10)]
    ">
      {/* Logo */}
      <div className="px-5 pt-7 pb-8">
        <h1 className="text-base font-bold text-white tracking-tight font-[DM_Sans]">
          PILOTAGE GLPI
        </h1>
        <p className="text-[10px] text-white/45 mt-0.5 tracking-[0.1em] uppercase">
          DSI CPAM 92
        </p>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 space-y-0.5">
        {NAV_ITEMS.map(({ icon: Icon, label, path }) => {
          const active = pathname === path;
          return (
            <Link
              key={path}
              to={path}
              className={`
                flex items-center gap-3 px-3 py-2.5 rounded-xl
                text-sm font-medium transition-all duration-150
                ${active
                  ? 'bg-white/15 text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]'
                  : 'text-white/65 hover:bg-white/8 hover:text-white'}
              `}
            >
              <Icon size={18} strokeWidth={active ? 2.2 : 1.8} />
              {label}
              {active && (
                <div className="ml-auto w-1.5 h-1.5 rounded-full bg-amber-400
                  shadow-[0_0_8px_rgba(255,202,40,0.5)]" />
              )}
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="px-5 py-4 border-t border-white/8">
        <p className="text-[10px] text-white/25">v1.0.0 · Tauri 2.10</p>
      </div>
    </aside>
  );
}
```

### 5.3. KPI Card

```tsx
// src/components/shared/KpiCard.tsx
import type { ReactNode } from 'react';

interface KpiCardProps {
  label: string;
  value: number | string;
  previousValue?: number;
  format?: 'number' | 'percent' | 'days';
  trend?: 'up' | 'down' | 'neutral';
  trendIsGood?: boolean;
  icon?: ReactNode;
  accentColor?: string;     // hex, ex: '#1565C0'
}

export function KpiCard({
  label, value, previousValue, format = 'number',
  trend, trendIsGood = true, icon, accentColor = '#0C419A',
}: KpiCardProps) {
  const displayValue = format === 'percent'
    ? `${value}%`
    : format === 'days'
      ? `${value}j`
      : typeof value === 'number'
        ? value.toLocaleString('fr-FR')
        : value;

  const delta = previousValue != null && typeof value === 'number'
    ? ((value - previousValue) / previousValue * 100).toFixed(1)
    : null;

  const isGood = trend === 'neutral' ? null
    : trend === 'up' ? trendIsGood
    : !trendIsGood;

  return (
    <div className="
      relative overflow-hidden bg-white rounded-2xl p-5
      shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
      hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]
      hover:-translate-y-0.5
      transition-all duration-200 ease-[cubic-bezier(0.4,0,0.2,1)]
      group
    ">
      {/* Barre d'accent top */}
      <div
        className="absolute top-0 inset-x-0 h-[3px] rounded-t-2xl"
        style={{ background: `linear-gradient(90deg, ${accentColor}, ${accentColor}88)` }}
      />

      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <span className="text-xs font-semibold uppercase tracking-wider text-slate-400 font-[DM_Sans]">
          {label}
        </span>
        {icon && (
          <div
            className="w-9 h-9 rounded-[10px] flex items-center justify-center"
            style={{ background: `${accentColor}12` }}
          >
            {icon}
          </div>
        )}
      </div>

      {/* Valeur + tendance */}
      <div className="flex items-baseline gap-2.5">
        <span className="text-[36px] font-bold font-[DM_Sans] tracking-tight leading-none text-slate-800">
          {displayValue}
        </span>
        {delta != null && trend && trend !== 'neutral' && (
          <span className={`
            text-xs font-semibold px-2 py-0.5 rounded-lg flex items-center gap-0.5
            ${isGood ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'}
          `}>
            {trend === 'up' ? '↑' : '↓'} {delta}%
          </span>
        )}
      </div>

      {/* Barre de progression subtile */}
      <div className="mt-4 h-[3px] rounded-full bg-slate-100 overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-700 ease-out"
          style={{
            background: `linear-gradient(90deg, ${accentColor}, ${accentColor}AA)`,
            width: `${Math.min(100, typeof value === 'number' ? (value / 10000) * 100 : 50)}%`,
          }}
        />
      </div>
    </div>
  );
}
```

### 5.4. Tableau KPI Material

```tsx
// src/components/shared/BilanTable.tsx
import type { ReactNode } from 'react';

interface BilanRow {
  label: string;
  value: number | string;
  highlight?: boolean;
  positive?: boolean;
  negative?: boolean;
  icon?: ReactNode;
}

export function BilanTable({ rows }: { rows: BilanRow[] }) {
  return (
    <div className="bg-white rounded-2xl overflow-hidden
      shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]">
      <table className="w-full">
        <thead>
          <tr className="border-b border-slate-100">
            <th className="text-left px-6 py-3.5 text-xs font-semibold
              uppercase tracking-wider text-slate-400 font-[DM_Sans]">
              Indicateur
            </th>
            <th className="text-right px-6 py-3.5 text-xs font-semibold
              uppercase tracking-wider text-slate-400 font-[DM_Sans]">
              Valeur
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i} className={`
              border-b border-slate-50 last:border-0
              transition-colors duration-100
              hover:bg-[rgba(12,65,154,0.04)]
              ${row.highlight ? 'bg-primary-50/50' : ''}
            `}>
              <td className="px-6 py-3.5 flex items-center gap-3">
                {row.icon && <span className="text-slate-400">{row.icon}</span>}
                <span className={`text-sm font-[Source_Sans_3]
                  ${row.highlight ? 'font-semibold text-slate-800' : 'text-slate-600'}`}>
                  {row.label}
                </span>
              </td>
              <td className={`px-6 py-3.5 text-right text-base font-semibold
                font-[DM_Sans] tabular-nums
                ${row.positive ? 'text-emerald-600' : ''}
                ${row.negative ? 'text-red-600' : ''}
                ${!row.positive && !row.negative ? 'text-slate-800' : ''}
              `}>
                {typeof row.value === 'number'
                  ? row.value.toLocaleString('fr-FR')
                  : row.value}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

### 5.5. Header de page

```tsx
// Pattern à utiliser dans chaque page
<header className="
  sticky top-0 z-10
  bg-[#F5F7FA]/80 backdrop-blur-sm
  px-8 pt-6 pb-4
  border-b border-slate-200/30
">
  <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800 tracking-tight">
    {pageTitle}
  </h1>
  <p className="text-sm text-slate-400 mt-1">
    {subtitle}
  </p>
</header>
```

---

## 6. Thème ECharts Material

### 6.1. Fichier thème complet

```typescript
// src/lib/echarts-theme.ts
import * as echarts from 'echarts/core';

export const CPAM_MATERIAL_THEME: echarts.ThemeOption = {
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

// Enregistrer au démarrage de l'app (main.tsx ou App.tsx)
echarts.registerTheme('cpam-material', CPAM_MATERIAL_THEME);
```

### 6.2. Utilisation dans useECharts

```typescript
// Passer le thème au hook existant
const { containerRef } = useECharts({
  option: bilanOption,
  theme: 'cpam-material',   // ← nom enregistré
  onEvents: { click: handleDrillDown },
});
```

### 6.3. Options spécifiques au graphique Bilan (Évolution des flux)

```typescript
const bilanOption: ECOption = {
  grid: {
    top: 60, right: 30, bottom: 80, left: 60,
    containLabel: false,
  },
  xAxis: {
    type: 'category',
    data: periods,
    axisLabel: { rotate: 35, fontSize: 11 },
  },
  yAxis: {
    type: 'value',
    axisLabel: {
      formatter: (v: number) => v >= 1000 ? `${(v/1000).toFixed(1)}k` : String(v),
    },
  },
  series: [
    {
      name: 'Entrants',
      type: 'bar',
      data: entrants,
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
      data: sortants,
      itemStyle: {
        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
          { offset: 0, color: '#43A047' },
          { offset: 1, color: '#2E7D32' },
        ]),
        borderRadius: [6, 6, 0, 0],
      },
    },
    {
      name: 'Stock net',
      type: 'line',
      data: stockNet,
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
  ],
};
```

---

## 7. Layout de page

### Structure type (toutes les pages)

```tsx
{/* Layout global — dans App.tsx ou MainLayout.tsx */}
<div className="flex h-screen bg-[#F5F7FA]">
  <Sidebar />

  <main className="flex-1 overflow-y-auto">
    {/* Header sticky avec backdrop-blur */}
    <header className="sticky top-0 z-10 bg-[#F5F7FA]/80 backdrop-blur-sm
      px-8 pt-6 pb-4 border-b border-slate-200/30">
      <h1 className="text-2xl font-bold font-[DM_Sans] text-slate-800">
        {pageTitle}
      </h1>
    </header>

    {/* Contenu scrollable */}
    <div className="px-8 pb-8 pt-6 space-y-6">
      {/* ... cards, graphiques, tableaux ... */}
    </div>
  </main>
</div>
```

### Grilles KPI par type de page

```tsx
{/* Page Stock / Bilan — 4 KPI côte à côte */}
<div className="grid grid-cols-4 gap-5">
  <KpiCard ... />
  <KpiCard ... />
  <KpiCard ... />
  <KpiCard ... />
</div>

{/* Page Catégories — 3 KPI */}
<div className="grid grid-cols-3 gap-5">
  <KpiCard ... />
  <KpiCard ... />
  <KpiCard ... />
</div>

{/* Page Text Mining — 2 larges */}
<div className="grid grid-cols-2 gap-5">
  <KpiCard ... />
  <KpiCard ... />
</div>
```

---

## 8. Spacing & border-radius

### Tokens de référence

| Token Tailwind | Valeur | Usage |
|----------------|--------|-------|
| `gap-5` | 20px | Gap dans grilles KPI |
| `p-4` | 16px | Padding card compact |
| `p-5` | 20px | Padding KPI cards |
| `p-6` | 24px | Padding card standard |
| `p-8` | 32px | Padding card large |
| `px-8` | 32px | Marges horizontales main content |
| `space-y-6` | 24px | Espacement vertical entre sections |
| `rounded-2xl` | 16px | Cards, conteneurs principaux |
| `rounded-xl` | 12px | Boutons nav, badges, éléments secondaires |
| `rounded-lg` | 8px | Inputs, petits boutons |
| `rounded-[10px]` | 10px | Icônes dans cards, boutons d'action |

### Règle fondamentale

- **Conteneur principal** (card, sidebar, popover) → `rounded-2xl` (16px)
- **Élément à l'intérieur** d'un conteneur → `rounded-xl` (12px) max
- **Sous-élément** (badge, tag, chip) → `rounded-lg` (8px)
- Jamais `rounded-md` (6px) ni `rounded` (4px) dans les nouveaux composants

---

## 9. Transitions & animations

### CSS à ajouter dans app.css

```css
@layer utilities {
  /* Animation d'entrée des sections */
  @keyframes fadeSlideUp {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .animate-fade-slide-up {
    animation: fadeSlideUp 500ms cubic-bezier(0.4, 0, 0.2, 1) both;
  }

  /* Stagger delay utilities */
  .animation-delay-150 { animation-delay: 150ms; }
  .animation-delay-300 { animation-delay: 300ms; }
  .animation-delay-450 { animation-delay: 450ms; }
}
```

### Règles d'animation

| Composant | Propriété | Durée | Easing |
|-----------|-----------|-------|--------|
| Card shadow | box-shadow | 200ms | `cubic-bezier(0.4, 0, 0.2, 1)` |
| Card lift (KPI) | transform | 200ms | `cubic-bezier(0.4, 0, 0.2, 1)` |
| Nav items | background, color | 150ms | `ease` |
| Ligne tableau hover | background | 100ms | `ease` |
| Apparition section | opacity + translateY | 500ms | `cubic-bezier(0.4, 0, 0.2, 1)` |
| Barre de progression | width | 700ms | `ease-out` |

### Pattern d'apparition staggerée

```tsx
{/* Chaque section a un délai incrémenté */}
<div className="animate-fade-slide-up">
  {/* KPI grid */}
</div>
<div className="animate-fade-slide-up animation-delay-150">
  {/* Chart */}
</div>
<div className="animate-fade-slide-up animation-delay-300">
  {/* Table */}
</div>
```

---

## 10. Checklist d'implémentation

### Phase 1 — Fondations (faire en premier)
- [ ] Télécharger DM Sans (500, 600, 700) + Source Sans 3 (400, 600) en woff2 dans `public/fonts/`
- [ ] Ajouter les `@font-face` dans `app.css`
- [ ] Remplacer le `@theme` complet dans `app.css` (section 2.1)
- [ ] Ajouter les keyframes et utilitaires d'animation (section 9)
- [ ] Créer `src/components/shared/Card.tsx`
- [ ] Créer `src/lib/echarts-theme.ts` + appeler `registerTheme` dans `main.tsx`

### Phase 2 — Layout global
- [ ] Refactorer `Sidebar.tsx` avec gradient + shadow-5 + dot amber
- [ ] Refactorer le layout principal (MainLayout ou App.tsx) avec `flex h-screen bg-[#F5F7FA]`
- [ ] Ajouter le header sticky backdrop-blur à chaque page

### Phase 3 — Composants
- [ ] Créer `KpiCard.tsx` avec barre d'accent, tendance, barre de progression
- [ ] Créer `BilanTable.tsx` (Material, hover, highlight)
- [ ] Remplacer les conteneurs `div` bruts par `<Card>` partout
- [ ] Supprimer tous les `border-gray-*` sur les conteneurs principaux

### Phase 4 — Graphiques
- [ ] Appliquer le thème `cpam-material` à tous les appels `useECharts`
- [ ] Ajouter les gradients sur les barres du BilanChart
- [ ] Ajouter le area fill semi-transparent sur la ligne Stock net
- [ ] Vérifier les tooltips (rounded-xl, shadow-3, padding 12/16)

### Phase 5 — Polish
- [ ] Ajouter `animate-fade-slide-up` staggeré sur chaque page
- [ ] Vérifier les contrastes WCAG AA sur chaque composant
- [ ] Tester la scrollbar custom sur Windows (Tauri/Webkit)
- [ ] Supprimer tout usage résiduel d'Inter/Arial dans les styles
