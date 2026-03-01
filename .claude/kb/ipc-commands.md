# Commandes IPC Tauri — GLPI Dashboard

Source : Segment 2

Toutes les commandes sont `async` (thread pool Tokio, ne bloquent pas l'UI).
Retour : `Result<T, String>` (String = message d'erreur sérialisé).
`serde_json` camelCase pour tous les types IPC (`#[serde(rename_all = "camelCase")]`).

---

## Import

### `import_csv`
```rust
#[tauri::command]
pub async fn import_csv(
    state: State<'_, AppState>,
    path: String,
    on_progress: Channel<ImportEvent>,  // Channel API Tauri 2
) -> Result<ImportResult, String>
```

**ImportEvent** (Channel) :
```rust
pub enum ImportEvent {
    Progress { rows_parsed: usize, total_estimated: usize, phase: String },
    // phase: "parsing" | "normalizing" | "inserting" | "indexing"
    Complete { duration_ms: u64, total_tickets: usize, vivants: usize, termines: usize },
    Warning { line: usize, message: String },
}
```

**ImportResult** (retour) :
```rust
pub struct ImportResult {
    pub import_id: i64,
    pub total_tickets: usize,
    pub vivants_count: usize,
    pub termines_count: usize,
    pub skipped_rows: usize,
    pub warnings: Vec<ParseWarning>,
    pub detected_columns: Vec<String>,
    pub missing_optional_columns: Vec<String>,
    pub unique_statuts: Vec<String>,
    pub parse_duration_ms: u64,
}
```

**Invocation TypeScript** :
```typescript
import { invoke, Channel } from '@tauri-apps/api/core';
const onProgress = new Channel<ImportEvent>();
onProgress.onmessage = (msg) => { /* progress */ };
const result = await invoke<ImportResult>('import_csv', { path, onProgress });
```

---

### `get_import_history`
```rust
pub async fn get_import_history(
    state: State<'_, AppState>,
) -> Result<Vec<ImportRecord>, String>
```

**ImportRecord** :
```rust
pub struct ImportRecord {
    pub id: i64,
    pub filename: String,
    pub import_date: String,
    pub total_rows: usize,
    pub vivants_count: usize,
    pub termines_count: usize,
    pub date_range_from: Option<String>,
    pub date_range_to: Option<String>,
    pub is_active: bool,
}
```

---

### `compare_imports`
```rust
pub async fn compare_imports(
    state: State<'_, AppState>,
    import_id_a: i64,
    import_id_b: i64,
) -> Result<ImportComparison, String>
```

**ImportComparison** :
```rust
pub struct ImportComparison {
    pub import_a: ImportRecord,
    pub import_b: ImportRecord,
    pub delta_total: i64,
    pub delta_vivants: i64,
    pub nouveaux_tickets: Vec<u64>,   // IDs apparus dans B mais pas dans A
    pub disparus_tickets: Vec<u64>,   // IDs dans A mais pas dans B
    pub delta_par_technicien: Vec<TechnicianDelta>,
}
```

---

## Stock

### `get_stock_overview`
```rust
pub async fn get_stock_overview(
    state: State<'_, AppState>,
) -> Result<StockOverview, String>
```

**StockOverview** :
```rust
pub struct StockOverview {
    pub total_vivants: usize,
    pub total_termines: usize,
    pub par_statut: Vec<StatutCount>,
    pub age_moyen_jours: f64,         // Moyenne vivants uniquement
    pub age_median_jours: f64,
    pub par_type: TypeBreakdown,      // incidents / demandes
    pub par_anciennete: Vec<AgeRangeCount>,  // >30j, >60j, >90j, >180j, >365j
    pub inactifs_14j: usize,
    pub inactifs_30j: usize,
}
```

---

### `get_stock_by_technician`
```rust
pub async fn get_stock_by_technician(
    state: State<'_, AppState>,
    filters: Option<StockFilters>,
) -> Result<Vec<TechnicianStock>, String>
```

**StockFilters** (entrée) :
```rust
pub struct StockFilters {
    pub statut: Option<String>,
    pub type_ticket: Option<String>,
    pub groupe: Option<String>,
    pub min_anciennete: Option<i64>,
    pub max_anciennete: Option<i64>,
}
```

**TechnicianStock** (sortie) :
```rust
pub struct TechnicianStock {
    pub technicien: String,
    pub total: usize,
    pub en_cours: usize,
    pub en_attente: usize,
    pub planifie: usize,
    pub nouveau: usize,
    pub incidents: usize,
    pub demandes: usize,
    pub age_moyen_jours: f64,
    pub inactifs_14j: usize,
    pub ecart_seuil: i64,        // total - seuil (négatif = sous le seuil)
    pub couleur_seuil: String,   // "vert" | "jaune" | "orange" | "rouge"
}
```

---

### `get_stock_by_group`
```rust
pub async fn get_stock_by_group(
    state: State<'_, AppState>,
    filters: Option<StockFilters>,
) -> Result<Vec<GroupStock>, String>
```

**GroupStock** :
```rust
pub struct GroupStock {
    pub groupe: String,
    pub groupe_niveau1: String,
    pub groupe_niveau2: Option<String>,
    pub total: usize,
    pub en_cours: usize,
    pub en_attente: usize,
    pub incidents: usize,
    pub demandes: usize,
    pub nb_techniciens: usize,
    pub age_moyen_jours: f64,
}
```

---

### `get_ticket_detail`
```rust
pub async fn get_ticket_detail(
    state: State<'_, AppState>,
    ticket_id: u64,
) -> Result<TicketDetail, String>
```

**TicketDetail** :
```rust
pub struct TicketDetail {
    pub id: u64,
    pub titre: String,
    pub statut: String,
    pub type_ticket: String,
    pub priorite: Option<u8>,
    pub urgence: Option<u8>,
    pub demandeur: String,
    pub techniciens: Vec<String>,
    pub groupes: Vec<String>,
    pub date_ouverture: String,
    pub derniere_modification: Option<String>,
    pub nombre_suivis: Option<u32>,
    pub suivis_description: String,
    pub solution: String,
    pub taches_description: String,
    pub anciennete_jours: Option<i64>,
    pub inactivite_jours: Option<i64>,
    pub action_recommandee: Option<String>,
    pub motif_classification: Option<String>,
    pub categorie: Option<String>,
}
```

---

### `get_technician_tickets`
```rust
pub async fn get_technician_tickets(
    state: State<'_, AppState>,
    technician: String,
    filters: Option<StockFilters>,
) -> Result<Vec<TicketSummary>, String>
```

**TicketSummary** :
```rust
pub struct TicketSummary {
    pub id: u64,
    pub titre: String,
    pub statut: String,
    pub type_ticket: String,
    pub technicien_principal: Option<String>,
    pub groupe_principal: Option<String>,
    pub date_ouverture: String,
    pub derniere_modification: Option<String>,
    pub anciennete_jours: Option<i64>,
    pub inactivite_jours: Option<i64>,
    pub nombre_suivis: Option<u32>,
    pub action_recommandee: Option<String>,
    pub motif_classification: Option<String>,
}
```

---

## Bilan Temporel

### `get_bilan_temporel`
```rust
pub async fn get_bilan_temporel(
    state: State<'_, AppState>,
    request: BilanRequest,
) -> Result<BilanTemporel, String>
```

**BilanRequest** (entrée) :
```rust
pub struct BilanRequest {
    pub period: String,              // "day" | "week" | "month"
    pub date_from: String,           // ISO 8601
    pub date_to: String,             // ISO 8601
    pub group_by: Option<String>,    // "technicien" | "groupe" | "type"
}
```

**BilanTemporel** (sortie) :
```rust
pub struct BilanTemporel {
    pub periodes: Vec<PeriodData>,
    pub totaux: BilanTotaux,
    pub ventilation: Option<Vec<BilanVentilation>>,
}

pub struct PeriodData {
    pub period_key: String,          // "2025-01-06" | "2025-S02" | "2025-01"
    pub period_label: String,        // "06/01/2025" | "Sem. 2" | "Janvier 2025"
    pub entrees: usize,
    pub sorties: usize,
    pub delta: i64,
    pub stock_cumule: Option<usize>,
}
```

---

## Catégories

### `get_categories_tree`
```rust
pub async fn get_categories_tree(
    state: State<'_, AppState>,
    request: CategoriesRequest,
) -> Result<CategoryTree, String>
```

**CategoriesRequest** (entrée) :
```rust
pub struct CategoriesRequest {
    pub scope: String,               // "vivants" | "tous" | "termines"
    pub source: Option<String>,      // "groupe" (défaut) | "categorie" (si dispo)
}
```

**CategoryTree** (sortie) :
```rust
pub struct CategoryTree {
    pub source: String,              // "groupe" | "categorie"
    pub nodes: Vec<CategoryNode>,
    pub total_tickets: usize,
}

pub struct CategoryNode {
    pub name: String,
    pub full_path: String,
    pub level: usize,
    pub count: usize,
    pub percentage: f64,
    pub incidents: usize,
    pub demandes: usize,
    pub age_moyen: f64,
    pub children: Vec<CategoryNode>,
}
```

---

## Data Mining

### `run_text_analysis`
```rust
pub async fn run_text_analysis(
    state: State<'_, AppState>,
    request: TextAnalysisRequest,
) -> Result<TextAnalysisResult, String>
```

**TextAnalysisRequest** :
```rust
pub struct TextAnalysisRequest {
    pub corpus: String,              // "titres" | "suivis" | "solutions" | "all"
    pub scope: String,               // "vivants" | "tous" | "termines"
    pub group_by: Option<String>,    // "groupe" | "technicien" | "categorie"
    pub top_n: Option<usize>,        // défaut 50
}
```

### `get_clusters`
```rust
pub async fn get_clusters(
    state: State<'_, AppState>,
    request: ClusterRequest,
) -> Result<ClusterResult, String>
```

**ClusterRequest** (entrée) :
```rust
pub struct ClusterRequest {
    pub k_min: usize,           // défaut 2
    pub k_max: usize,           // défaut 10
    pub n_iterations: usize,    // défaut 100
    pub corpus: String,         // "titres" | "suivis" | "all"
}
```

**ClusterResult** (sortie) :
```rust
pub struct ClusterResult {
    pub k_optimal: usize,
    pub silhouette_score: f64,
    pub clusters: Vec<ClusterInfo>,
    pub duration_ms: u64,
}

pub struct ClusterInfo {
    pub id: usize,
    pub label: String,           // 5 mots-clés representatifs concaténés
    pub keywords: Vec<String>,   // Top 5 mots-clés TF-IDF
    pub ticket_ids: Vec<u64>,
    pub size: usize,
}
```

### `detect_anomalies`
```rust
pub async fn detect_anomalies(
    state: State<'_, AppState>,
) -> Result<Vec<AnomalyAlert>, String>
```

**AnomalyAlert** (sortie) :
```rust
pub struct AnomalyAlert {
    pub ticket_id: u64,
    pub titre: String,
    pub anomaly_type: String,    // "delai_anormal" | "categorie_inhabituelle" | "dormant"
    pub z_score: f64,
    pub value_days: f64,         // délai effectif en jours
    pub technicien: Option<String>,
    pub groupe: Option<String>,
}
```

---

## Export

### `export_excel_stock`
```rust
pub async fn export_excel_stock(
    state: State<'_, AppState>,
) -> Result<ExportResult, String>
```

### `export_excel_bilan`
```rust
pub async fn export_excel_bilan(
    state: State<'_, AppState>,
    request: BilanRequest,
) -> Result<ExportResult, String>
```

### `export_excel_plan_action`
```rust
pub async fn export_excel_plan_action(
    state: State<'_, AppState>,
    technician: String,
) -> Result<ExportResult, String>
```

**ExportResult** :
```rust
pub struct ExportResult {
    pub path: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
}
```

---

## Config

### `get_config`
```rust
pub async fn get_config(
    state: State<'_, AppState>,
) -> Result<AppConfig, String>
```

### `update_config`
```rust
pub async fn update_config(
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<(), String>
```

**AppConfig** :
```rust
pub struct AppConfig {
    pub seuil_tickets_technicien: u32,    // défaut 20
    pub seuil_anciennete_cloturer: u32,   // défaut 90
    pub seuil_inactivite_cloturer: u32,   // défaut 60
    pub seuil_anciennete_relancer: u32,   // défaut 30
    pub seuil_inactivite_relancer: u32,   // défaut 14
    pub seuil_couleur_vert: u32,          // défaut 10
    pub seuil_couleur_jaune: u32,         // défaut 20
    pub seuil_couleur_orange: u32,        // défaut 40
    pub statuts_vivants: Vec<String>,
    pub statuts_termines: Vec<String>,
}
```
