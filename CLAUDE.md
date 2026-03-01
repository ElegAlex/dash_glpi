# GLPI Dashboard

## Stack
Tauri 2.10+ (Rust) + React 19 + TypeScript + Tailwind CSS 4

## Build
cargo tauri dev          # dev mode
cargo tauri build        # production .exe
cargo test               # tests Rust
pnpm build               # build frontend
pnpm dev                 # dev frontend seul

## Conventions Rust
- Erreurs : thiserror pour les types d'erreur, Result<T, String> pour les commandes Tauri
- Naming : snake_case partout, structs CamelCase
- Tests : dans le même fichier (#[cfg(test)] mod tests) ou fichier _test.rs adjacent
- Documentation : rustdoc sur les fonctions pub

## Conventions React/TypeScript
- Composants : PascalCase, un fichier par composant
- Hooks : use[Nom].ts dans hooks/
- Types : dans types/, miroir des structs Rust
- Pas de any. Typer toutes les props et retours invoke.
- Tailwind CSS 4 uniquement pour le styling, pas de CSS custom
- Config Tailwind en CSS (@import "tailwindcss"), pas de tailwind.config.js

## Routing
- Package : react-router (PAS react-router-dom)
- import { BrowserRouter, Routes, Route } from 'react-router'

## IPC
- Commandes Rust : #[tauri::command] dans commands.rs
- Appels frontend : import { invoke } from '@tauri-apps/api/core'
- Toujours typer le retour : invoke<MonType>('ma_commande', { arg })

## Exports Excel
- Crate : rust_xlsxwriter 0.93+
- Toujours : filtres auto, freeze panes, largeurs colonnes explicites
- Couleurs seuil : vert ≤10, jaune 11-20, orange 21-40, rouge >40

---

## Design System — Material Design Épuré

> Référence complète : `docs/DESIGN_SYSTEM.md`
> Style : Material Design épuré — élévation par les ombres, couleurs vives, typographie hiérarchique.

### Principe fondamental
Les surfaces se distinguent par leur **profondeur (ombres)**, PAS par des bordures.
Jamais de `border-gray-200` pour séparer des blocs — utiliser les ombres Material à la place.

### Fonts (offline, @font-face dans app.css)
- **Titres, labels, KPI** : `DM Sans` (500, 600, 700) — `font-[DM_Sans]`
- **Texte courant, tooltips** : `Source Sans 3` (400, 600) — `font-[Source_Sans_3]`
- **Code** : `JetBrains Mono`
- JAMAIS Inter, Roboto, Arial, ou system-ui seul

### Couleurs principales
| Token | Hex | Usage |
|-------|-----|-------|
| primary-500 | #0C419A | Sidebar, boutons actifs, accents |
| primary-700 | #082A66 | Bas du gradient sidebar |
| accent-500 | #FFC107 | Highlights, badges, dot actif sidebar |
| accent-700 | #FF8F00 | Ligne Stock net sur les graphiques |
| surface-base | #F5F7FA | Fond global de l'app (body) |
| surface-card | #FFFFFF | Toutes les cards |
| text-primary | #1E293B | Texte principal (slate-800) |
| text-secondary | #64748B | Labels, sous-titres (slate-500) |
| text-muted | #94A3B8 | Labels tertiaires (slate-400) |

### Couleurs sémantiques
| Token | Hex | Usage |
|-------|-----|-------|
| success | #2E7D32 | Tendances positives, sortants |
| error | #C62828 | Tendances négatives, alertes |
| warning | #E65100 | Seuils intermédiaires |

### Palette graphiques ECharts (dans l'ordre)
`#1565C0` `#2E7D32` `#FF8F00` `#6A1B9A` `#00838F` `#C62828` `#4E342E` `#37474F`

### Système d'ombres (5 niveaux)
```
shadow-1: 0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06)   → Cards au repos
shadow-2: 0 3px 6px rgba(0,0,0,0.10), 0 2px 4px rgba(0,0,0,0.06)   → Cards hover
shadow-3: 0 10px 20px rgba(0,0,0,0.10), 0 3px 6px rgba(0,0,0,0.06) → Dropdowns, popovers
shadow-4: 0 15px 30px rgba(0,0,0,0.12), 0 5px 15px rgba(0,0,0,0.08) → Modales
shadow-5: 0 20px 40px rgba(0,0,0,0.14), 0 8px 20px rgba(0,0,0,0.10) → Sidebar
```

### Règles impératives pour chaque composant

**Cards (TOUTES les cards dans l'app)** :
- `bg-white rounded-2xl shadow-[shadow-1]` — TOUJOURS rounded-2xl (16px), JAMAIS rounded-lg
- Hover interactif : `hover:shadow-[shadow-2] transition-shadow duration-200`
- Padding standard : `p-6`, large : `p-8`
- PAS de border. L'ombre suffit.

**Sidebar** :
- `bg-gradient-to-b from-[#0C419A] to-[#082A66]` + shadow-5
- Items nav : `rounded-xl` (12px), actif = `bg-white/15`, hover = `bg-white/8`
- Item actif : dot amber `w-1.5 h-1.5 rounded-full bg-amber-400` aligné à droite
- Largeur : `w-60` (240px)

**KPI Cards** :
- Barre d'accent colorée en haut (3px, couleur spécifique au KPI)
- Valeur héros : `text-4xl font-bold font-[DM_Sans] tracking-tight`
- Label : `text-xs font-semibold uppercase tracking-wider text-slate-400`
- Badge tendance : fond vert/rouge clair + texte vert/rouge foncé, `rounded-lg px-2 py-0.5`
- Barre de progression subtile en bas (3px, gradient de la couleur d'accent)
- Hover : translateY(-2px) + shadow-2

**Tableaux** :
- Wrapper : `bg-white rounded-2xl overflow-hidden shadow-[shadow-1]`
- En-tête : `text-xs font-semibold uppercase tracking-wider text-slate-400`
- Lignes : hover `bg-primary-500/[0.04]`, PAS de zebra striping
- Ligne highlight : `bg-primary-50/50`
- Valeurs numériques : `font-[DM_Sans] font-semibold tabular-nums`
- Valeurs positives : `text-emerald-600`, négatives : `text-red-600`

**Graphiques ECharts** :
- Thème enregistré : `cpam-material` (voir docs/DESIGN_SYSTEM.md section 6)
- Barres : coins arrondis `borderRadius: [6, 6, 0, 0]` + gradient vertical
- Ligne stock net : `smooth: true`, épaisseur 3, dots avec bordure blanche
- Area fill : gradient vertical semi-transparent (15% → 2% opacity)
- Tooltip : `rounded-xl shadow-3 bg-white border-slate-200`
- Grille : lignes horizontales uniquement, `dashed`, couleur `#F1F5F9`
- Pas de axisLine sur Y, pas de tickLine

**Header de page** :
- Sticky top avec backdrop-blur : `sticky top-0 bg-[#F5F7FA]/80 backdrop-blur-sm`
- Titre : `text-2xl font-bold font-[DM_Sans]`
- Sous-titre : `text-sm text-slate-400`

**Layout global** :
- `<div className="flex h-screen bg-[#F5F7FA]">`
- Contenu : `flex-1 overflow-y-auto`, padding `px-8 pb-8`
- Espacement vertical entre sections : `space-y-6`
- Grille KPI : `grid grid-cols-4 gap-5`

### Animations
- Cards : `transition-shadow duration-200 ease-[cubic-bezier(0.4,0,0.2,1)]`
- Nav items : `transition-all duration-150 ease`
- Apparition au chargement : fade-in + translateY(12px→0) sur 500ms, staggeré de 150ms entre sections
- Scrollbar custom : 6px, thumb `slate-300/60`, track transparent
