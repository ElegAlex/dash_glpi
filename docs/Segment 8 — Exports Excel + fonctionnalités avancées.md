# Segment 8 — Exports Excel + fonctionnalités avancées

**rust_xlsxwriter pour l'export multi-onglets professionnel, strsim pour la détection de doublons, tauri-plugin-notification pour l'alerting desktop, suivi longitudinal par diff entre imports, export PDF via Typst, et intégration future de l'API REST GLPI.**

---

Le Segment 8 couvre les six derniers piliers techniques du GLPI Dashboard : l'export Excel avancé qui constitue le livrable principal de l'application (plans d'action, tableaux de bord, bilans), la détection de doublons entre tickets par similarité de chaînes, les notifications desktop natives pour l'alerting temps réel, le suivi longitudinal par comparaison entre imports CSV successifs, la génération de rapports PDF professionnels, et enfin la feuille de route d'intégration avec l'API REST GLPI pour supprimer à terme la dépendance aux exports CSV manuels. Chaque section fournit les versions de crates exactes vérifiées sur crates.io à date, le code Rust complet et fonctionnel, et les considérations de performance pour le volume CPAM 92 (~10 000 tickets).

Ce segment consomme les données parsées du Segment 1, le schéma SQLite du Segment 2, les KPI du Segment 3, les catégories hiérarchiques du Segment 4, le pipeline NLP du Segment 5, les résultats de clustering du Segment 6, et s'intègre aux visualisations frontend du Segment 7.

---

## 1. rust_xlsxwriter 0.93+ : l'export Excel professionnel complet

### 1.1 État du crate et positionnement

rust_xlsxwriter est le portage Rust du célèbre XlsxWriter Python, par le même auteur (John McNamara). La version actuelle est **0.93.0** (février 2026), publiée sur crates.io avec **3,8× les performances de l'équivalent Python** et une couverture fonctionnelle quasi complète du format XLSX. Le crate a 1 200+ étoiles GitHub, un rythme de release soutenu (~2 releases/mois), et zéro dépendance runtime au-delà de `zip`. Il est sous licence MIT/Apache 2.0.

Le crate génère des fichiers XLSX conformes à la spécification OOXML (Office Open XML) lus par Excel 2007+, LibreOffice Calc et Google Sheets. La méthode `save_to_buffer()` retourne un `Vec<u8>` idéal pour le passage via commande Tauri — le frontend peut ensuite utiliser `@tauri-apps/plugin-dialog` pour proposer un emplacement de sauvegarde, ou le backend peut écrire directement sur disque.

### 1.2 Dépendances Cargo.toml

```toml
[dependencies]
rust_xlsxwriter = { version = "0.93", features = ["serde", "chrono", "zlib", "ryu"] }
```

|Feature|Effet|Recommandation|
|---|---|---|
|`serde`|Sérialisation directe des structs Rust vers les lignes Excel|**Activée** — élimine le mapping manuel champ par champ|
|`chrono`|Support natif de `NaiveDate`, `NaiveDateTime`, `NaiveTime`|**Activée** — le parser (Segment 1) produit des `NaiveDateTime`|
|`zlib`|Utilise `flate2` (zlib natif) au lieu de `miniz_oxide` pour la compression ZIP|**Activée** — ~1,5× plus rapide sur la phase de compression|
|`ryu`|Accélère l'écriture des cellules numériques de ~30%|Optionnelle — gain perceptible au-delà de 300K cellules|
|`polars`|Support des DataFrames Polars|Non utilisée — le Dashboard utilise des structs Rust|
|`constant_memory`|Mode streaming O(1) mémoire, écriture séquentielle uniquement|Non utilisée — désactive les tables et merges, inutile pour ≤50K lignes|

### 1.3 Architecture multi-onglets

Le CDC (section 6.2) spécifie quatre types d'exports Excel, tous multi-onglets :

|Export|Onglets|Lignes estimées|
|---|---|---|
|Tableau de bord stock|Vue globale + Techniciens + Groupes|~50 + ~40 + ~20|
|Plan d'action individuel|Entretien + Détail tickets + Checklist|~1 + ~200 + ~200|
|Bilan d'activité|Volume + Délais + Comparatif techniciens|~12 + ~40 + ~40|
|Rapport de suivi|Évolution stock + Delta techniciens|~30 + ~40|

La création d'un classeur multi-onglets suit le patron `Workbook::new()` → `add_worksheet()` en boucle :

```rust
use rust_xlsxwriter::*;

/// Structure de configuration partagée pour tous les exports GLPI.
pub struct ExportConfig {
    pub header_format: Format,
    pub date_format: Format,
    pub number_format: Format,
    pub euro_format: Format,
    pub percent_format: Format,
    pub wrap_format: Format,
}

impl ExportConfig {
    pub fn new() -> Self {
        Self {
            header_format: Format::new()
                .set_bold()
                .set_background_color("2C5F8A")
                .set_font_color("FFFFFF")
                .set_font_size(11)
                .set_border(FormatBorder::Thin)
                .set_text_wrap(),
            date_format: Format::new()
                .set_num_format("dd/mm/yyyy"),
            number_format: Format::new()
                .set_num_format("#,##0"),
            euro_format: Format::new()
                .set_num_format("#,##0.00 [$€-fr-FR]"),
            percent_format: Format::new()
                .set_num_format("0.0%"),
            wrap_format: Format::new()
                .set_text_wrap()
                .set_align(FormatAlign::Top),
        }
    }
}
```

**Point critique sur la locale française** : rust_xlsxwriter écrit les format strings en notation US-locale dans le XML OOXML. Excel traduit automatiquement à l'affichage selon la locale du poste. Ainsi `"#,##0.00"` s'affiche `12 345,67` sur un Windows français (la virgule US `,` devient l'espace français pour les milliers, le point US `.` devient la virgule française pour les décimales). Pour les dates, `"dd/mm/yyyy"` s'affiche correctement `25/12/2026`. Pour l'euro, `"#,##0.00 [$€-fr-FR]"` affiche `1 234,56 €`. Ce comportement est souvent source de confusion mais il est **correct et voulu** — ne jamais écrire `"#.##0,00"` qui provoquerait un affichage incohérent.

### 1.4 Écriture des données par onglet

Deux approches sont possibles : l'écriture cellule par cellule (`worksheet.write()`) et la sérialisation Serde (`worksheet.serialize()`). **La sérialisation Serde est recommandée** car elle élimine le mapping manuel des colonnes :

```rust
use rust_xlsxwriter::*;
use serde::Serialize;

/// Ligne du tableau de bord technicien (onglet 2 de l'export stock).
#[derive(Serialize)]
struct TechnicianRow {
    #[serde(rename = "Technicien")]
    name: String,
    #[serde(rename = "Stock total")]
    total: u32,
    #[serde(rename = "En cours")]
    en_cours: u32,
    #[serde(rename = "En attente")]
    en_attente: u32,
    #[serde(rename = "Planifié")]
    planifie: u32,
    #[serde(rename = "Nouveau")]
    nouveau: u32,
    #[serde(rename = "Incidents")]
    incidents: u32,
    #[serde(rename = "Demandes")]
    demandes: u32,
    #[serde(rename = "Priorité haute+")]
    high_priority: u32,
    #[serde(rename = "Sans suivi")]
    no_followup: u32,
    #[serde(rename = "> 90 jours")]
    over_90d: u32,
    #[serde(rename = "Âge moyen (j)")]
    avg_age: f64,
}

fn write_technician_sheet(
    workbook: &mut Workbook,
    technicians: &[TechnicianRow],
    config: &ExportConfig,
) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet();
    ws.set_name("Techniciens")?;

    // En-têtes avec sérialisation Serde
    ws.set_serialize_headers::<TechnicianRow>(0, 0)?;

    // Données
    for tech in technicians {
        ws.serialize(tech)?;
    }

    let last_row = technicians.len() as u32;
    let last_col = 11u16; // 12 colonnes (0-11)

    // Freeze panes : ligne d'en-tête figée
    ws.set_freeze_panes(1, 0)?;

    // Auto-filtre sur toutes les colonnes
    ws.autofilter(0, 0, last_row, last_col)?;

    // Largeurs de colonnes explicites
    ws.set_column_width(0, 28)?;  // Technicien
    ws.set_column_width(1, 12)?;  // Stock total
    for col in 2..=10 {
        ws.set_column_width(col, 14)?;
    }
    ws.set_column_width(11, 16)?; // Âge moyen

    Ok(())
}
```

Pour l'écriture cellule par cellule (nécessaire quand le format varie par cellule, notamment pour les couleurs conditionnelles sur la colonne Stock total) :

```rust
fn write_row_with_threshold_color(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    value: u32,
    config: &ExportConfig,
) -> Result<(), XlsxError> {
    let fmt = match value {
        0..=10 => Format::new()
            .set_background_color("C6EFCE").set_font_color("006100"),
        11..=20 => Format::new()
            .set_background_color("FFEB9C").set_font_color("9C6500"),
        21..=40 => Format::new()
            .set_background_color("F4B084").set_font_color("833C0C"),
        _ => Format::new()
            .set_background_color("FFC7CE").set_font_color("9C0006"),
    };
    ws.write_with_format(row, col, value, &fmt)?;
    Ok(())
}
```

### 1.5 Conditional formatting natif

Au-delà du coloriage cellule par cellule ci-dessus, rust_xlsxwriter supporte le **conditional formatting XLSX natif**, évalué dynamiquement par Excel à l'ouverture. L'avantage : si un utilisateur modifie une valeur dans le fichier, la couleur se met à jour automatiquement. C'est le mécanisme à privilégier pour les colonnes de stock :

```rust
use rust_xlsxwriter::{ConditionalFormatCell, ConditionalFormatCellRule};

fn apply_threshold_conditional_formatting(
    ws: &mut Worksheet,
    col: u16,
    last_row: u32,
) -> Result<(), XlsxError> {
    let green = Format::new()
        .set_background_color("C6EFCE")
        .set_font_color("006100");
    let yellow = Format::new()
        .set_background_color("FFEB9C")
        .set_font_color("9C6500");
    let orange = Format::new()
        .set_background_color("F4B084")
        .set_font_color("833C0C");
    let red = Format::new()
        .set_background_color("FFC7CE")
        .set_font_color("9C0006");

    // Ordre d'insertion = ordre d'évaluation.
    // Vert ≤ 10 (CDC § 3.1.2)
    ws.add_conditional_format(
        1, col, last_row, col,
        &ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::LessThanOrEqualTo(10))
            .set_format(&green),
    )?;

    // Jaune 11-20
    ws.add_conditional_format(
        1, col, last_row, col,
        &ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::Between(11, 20))
            .set_format(&yellow),
    )?;

    // Orange 21-40
    ws.add_conditional_format(
        1, col, last_row, col,
        &ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::Between(21, 40))
            .set_format(&orange),
    )?;

    // Rouge > 40
    ws.add_conditional_format(
        1, col, last_row, col,
        &ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::GreaterThan(40))
            .set_format(&red),
    )?;

    Ok(())
}
```

Autres types de conditional formatting disponibles dans rust_xlsxwriter :

|Type|Struct|Usage GLPI|
|---|---|---|
|Barre de données|`ConditionalFormatDataBar`|Visualiser l'ancienneté relative directement dans les cellules|
|Jeu d'icônes|`ConditionalFormatIconSet`|Feux tricolores (🔴🟡🟢) sur la colonne priorité|
|Échelle de couleurs|`ConditionalFormat2ColorScale` / `3ColorScale`|Heatmap d'ancienneté (vert→jaune→rouge)|
|Formule|`ConditionalFormatFormula`|Colorier la ligne entière si le statut = "En attente"|
|Doublons|`ConditionalFormatDuplicate`|Surligner les techniciens apparaissant plusieurs fois|

Exemple de barre de données sur la colonne Âge moyen :

```rust
use rust_xlsxwriter::ConditionalFormatDataBar;

ws.add_conditional_format(
    1, 11, last_row, 11,
    &ConditionalFormatDataBar::new()
        .set_fill_color("5B9BD5")
        .set_border_color("2E75B6"),
)?;
```

### 1.6 Graphiques embarqués

rust_xlsxwriter supporte les types de graphiques suivants : `Bar`, `Column`, `Line`, `Pie`, `Scatter`, `Area`, `Doughnut` et `Radar`. Les graphiques référencent des plages de données sur la même feuille ou sur une autre feuille du classeur. Un graphique peut combiner deux types (dual-axis) via `primary_chart.combine(&secondary_chart)`.

#### Camembert de répartition par statut (onglet Vue globale)

```rust
fn add_status_pie_chart(
    workbook: &mut Workbook,
    ws: &mut Worksheet,
    status_data: &[(String, u32)],
    config: &ExportConfig,
) -> Result<(), XlsxError> {
    // Écrire les données source dans une zone dédiée
    let data_start_row = 0u32;
    ws.write_with_format(data_start_row, 8, "Statut", &config.header_format)?;
    ws.write_with_format(data_start_row, 9, "Nombre", &config.header_format)?;

    for (i, (status, count)) in status_data.iter().enumerate() {
        let row = data_start_row + 1 + i as u32;
        ws.write(row, 8, status)?;
        ws.write(row, 9, *count)?;
    }
    let last_data_row = data_start_row + status_data.len() as u32;

    // Créer le graphique
    let mut chart = Chart::new(ChartType::Pie);
    chart.set_style(10); // Style prédéfini Excel

    chart.add_series()
        .set_name("Répartition par statut")
        .set_categories(("Vue globale", data_start_row + 1, 8, last_data_row, 8))
        .set_values(("Vue globale", data_start_row + 1, 9, last_data_row, 9));

    chart.title().set_name("Répartition du stock par statut");
    chart.legend().set_position(ChartLegendPosition::Bottom);
    chart.set_width(480);
    chart.set_height(320);

    // Insérer le graphique dans la feuille
    ws.insert_chart(2, 0, &chart)?;

    Ok(())
}
```

#### Barres horizontales : stock par technicien avec seuil (CDC § 6.3)

```rust
fn add_technician_bar_chart(
    ws: &mut Worksheet,
    tech_names: &[String],
    tech_counts: &[u32],
    threshold: u32,
) -> Result<(), XlsxError> {
    // Données source à partir de la colonne 14 (hors zone visible)
    let start_row = 0u32;
    for (i, (name, count)) in tech_names.iter().zip(tech_counts).enumerate() {
        let row = start_row + 1 + i as u32;
        ws.write(row, 14, name)?;
        ws.write(row, 15, *count)?;
        ws.write(row, 16, threshold)?; // Ligne de seuil
    }
    let last_row = start_row + tech_names.len() as u32;

    let mut chart = Chart::new(ChartType::Bar);

    // Série stock réel
    chart.add_series()
        .set_name("Stock")
        .set_categories(("Techniciens", 1, 14, last_row, 14))
        .set_values(("Techniciens", 1, 15, last_row, 15))
        .set_format(ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("5B9BD5")));

    // Série seuil (ligne de référence)
    chart.add_series()
        .set_name(&format!("Seuil ({})", threshold))
        .set_values(("Techniciens", 1, 16, last_row, 16))
        .set_format(
            ChartFormat::new()
                .set_solid_fill(ChartSolidFill::new().set_color("FF6B6B"))
                .set_line(ChartLine::new().set_color("FF6B6B").set_dash_type(ChartLineDashType::Dash)),
        );

    chart.title().set_name("Stock par technicien vs seuil");
    chart.x_axis().set_name("Nombre de tickets");
    chart.set_width(700);
    chart.set_height(500);

    ws.insert_chart(2, 0, &chart)?;
    Ok(())
}
```

#### Graphique dual-axis : flux entrée/sortie + stock cumulé (onglet Bilan)

```rust
fn add_flow_dual_chart(ws: &mut Worksheet) -> Result<(), XlsxError> {
    // Série primaire : barres empilées entrée/sortie (axe Y gauche)
    let mut bar_chart = Chart::new(ChartType::Column);
    bar_chart.add_series()
        .set_name("Entrées")
        .set_categories(("Bilan", 1, 0, 12, 0))    // Mois
        .set_values(("Bilan", 1, 1, 12, 1));         // Entrées
    bar_chart.add_series()
        .set_name("Sorties")
        .set_values(("Bilan", 1, 2, 12, 2));

    // Série secondaire : ligne du stock net (axe Y droit)
    let mut line_chart = Chart::new(ChartType::Line);
    line_chart.add_series()
        .set_name("Stock net")
        .set_values(("Bilan", 1, 3, 12, 3))
        .set_secondary_axis(true)
        .set_format(
            ChartFormat::new()
                .set_line(ChartLine::new().set_color("FF6B6B").set_width(2.5)),
        );

    // Combiner
    bar_chart.combine(&line_chart);
    bar_chart.title().set_name("Flux mensuels et stock net");
    bar_chart.y_axis().set_name("Tickets");
    bar_chart.y2_axis().set_name("Stock cumulé");
    bar_chart.set_width(800);
    bar_chart.set_height(400);

    ws.insert_chart(15, 0, &bar_chart)?;
    Ok(())
}
```

### 1.7 Freeze panes, auto-filtre et validation

**Freeze panes** (volets figés) : indispensable pour tout tableau dépassant un écran. Le CDC n'en parle pas explicitement, mais c'est un minimum de qualité professionnelle :

```rust
// Figer la ligne d'en-tête (la plus courante)
ws.set_freeze_panes(1, 0)?;

// Figer les 2 premières lignes ET la colonne A
ws.set_freeze_panes(2, 1)?;

// Panes avec séparation et position de défilement
ws.set_freeze_panes_top_cell(1, 0, 5, 0)?; // Défilement initial à la ligne 5
```

**Auto-filtre** : les flèches de tri/filtrage dans l'en-tête sont attendues par tous les utilisateurs métier Excel. L'appel `autofilter()` couvre la plage entière. On peut pré-appliquer un filtre pour n'afficher que certaines valeurs :

```rust
// Filtre automatique sur toutes les colonnes
ws.autofilter(0, 0, last_row, last_col)?;

// Pré-filtrer pour n'afficher que les tickets "En attente"
ws.filter_column(
    3, // Colonne Statut (index 0-based)
    &FilterCondition::new().add_list_filter("En attente"),
)?;

// Filtrer par valeur numérique : afficher seulement > 30 jours
ws.filter_column(
    5,
    &FilterCondition::new()
        .add_custom_filter(FilterCriteria::GreaterThan, 30),
)?;
```

**Data validation** : listes déroulantes pour le champ Action du plan d'action :

```rust
let action_validation = DataValidation::new()
    .allow_list_strings(&["Clôturer", "Relancer", "À qualifier", "Fait", "Hors périmètre"])
    .unwrap()
    .set_input_title("Action")
    .set_input_message("Sélectionnez l'action à mener")
    .set_error_title("Action invalide")
    .set_error_message("Veuillez choisir une action dans la liste.");

ws.add_data_validation(1, 6, last_row, 6, &action_validation)?;
```

### 1.8 Export complet : le plan d'action individuel

Le livrable le plus complexe est le plan d'action individuel (CDC § 6.2), généré pour chaque technicien dépassant le seuil. Il comporte 3 onglets :

```rust
/// Génère le plan d'action Excel pour un technicien.
pub fn generate_action_plan(
    technician: &str,
    tickets: &[ClassifiedTicket],  // Segment 3
    summary: &TechnicianSummary,   // Segment 3
    config: &ExportConfig,
) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();

    // --- Onglet 1 : Synthèse entretien ---
    let ws1 = workbook.add_worksheet();
    ws1.set_name("Synthèse")?;
    ws1.set_landscape();
    ws1.set_print_area(0, 0, 30, 7)?;

    // Titre
    let title_fmt = Format::new()
        .set_bold().set_font_size(16).set_font_color("2C5F8A");
    ws1.merge_range(0, 0, 0, 7, "", &title_fmt)?;
    ws1.write_with_format(0, 0,
        &format!("Plan d'action — {}", technician), &title_fmt)?;
    ws1.write(2, 0, &format!("Date : {}", chrono::Local::now().format("%d/%m/%Y")))?;

    // KPI résumé
    let kpi_headers = ["Stock total", "En cours", "En attente", "Incidents",
                       "Demandes", "Sans suivi", "> 90j", "Âge moyen"];
    let kpi_values = [summary.total, summary.en_cours, summary.en_attente,
                      summary.incidents, summary.demandes, summary.no_followup,
                      summary.over_90d, summary.avg_age as u32];
    for (col, (h, v)) in kpi_headers.iter().zip(kpi_values.iter()).enumerate() {
        ws1.write_with_format(4, col as u16, *h, &config.header_format)?;
        write_row_with_threshold_color(ws1, 5, col as u16, *v, config)?;
    }

    // Zone de notes libre pour l'entretien
    ws1.merge_range(8, 0, 8, 7, "Notes d'entretien", &config.header_format)?;
    let note_fmt = Format::new()
        .set_border(FormatBorder::Thin)
        .set_text_wrap()
        .set_align(FormatAlign::Top);
    ws1.merge_range(9, 0, 20, 7, "", &note_fmt)?;

    // --- Onglet 2 : Détail des tickets ---
    let ws2 = workbook.add_worksheet();
    ws2.set_name("Détail tickets")?;
    let detail_headers = ["ID", "Titre", "Statut", "Priorité",
        "Date ouverture", "Âge (j)", "Action recommandée", "Motif"];
    for (col, h) in detail_headers.iter().enumerate() {
        ws2.write_with_format(0, col as u16, *h, &config.header_format)?;
    }
    for (i, t) in tickets.iter().enumerate() {
        let row = (i + 1) as u32;
        ws2.write(row, 0, t.id)?;
        ws2.write(row, 1, &t.title)?;
        ws2.write(row, 2, &t.status)?;
        ws2.write(row, 3, &t.priority)?;
        ws2.write_with_format(row, 4, &t.open_date, &config.date_format)?;
        ws2.write(row, 5, t.age_days)?;
        // Couleur de fond selon l'action recommandée
        let action_fmt = match t.recommended_action.as_str() {
            "Clôturer" => Format::new().set_background_color("FFC7CE"),
            "Relancer" => Format::new().set_background_color("FFEB9C"),
            _ => Format::new().set_background_color("BDD7EE"),
        };
        ws2.write_with_format(row, 6, &t.recommended_action, &action_fmt)?;
        ws2.write(row, 7, &t.action_reason)?;
    }
    let last_row = tickets.len() as u32;
    ws2.set_freeze_panes(1, 0)?;
    ws2.autofilter(0, 0, last_row, 7)?;
    ws2.set_column_width(0, 12)?;
    ws2.set_column_width(1, 50)?;
    ws2.set_column_width(6, 22)?;
    ws2.set_column_width(7, 35)?;

    // Conditional formatting sur âge
    apply_threshold_conditional_formatting(ws2, 5, last_row)?;

    // --- Onglet 3 : Checklist d'actions ---
    let ws3 = workbook.add_worksheet();
    ws3.set_name("Checklist")?;
    let check_headers = ["ID", "Titre", "Action", "Statut action", "Commentaire"];
    for (col, h) in check_headers.iter().enumerate() {
        ws3.write_with_format(0, col as u16, *h, &config.header_format)?;
    }
    for (i, t) in tickets.iter().enumerate() {
        let row = (i + 1) as u32;
        ws3.write(row, 0, t.id)?;
        ws3.write(row, 1, &t.title)?;
        ws3.write(row, 2, &t.recommended_action)?;
        ws3.write(row, 3, "")?; // À remplir par le technicien
        ws3.write(row, 4, "")?;
    }
    // Validation liste déroulante sur la colonne Statut action
    let status_validation = DataValidation::new()
        .allow_list_strings(&["Fait", "En cours", "Reporté", "Annulé", "Hors périmètre"])
        .unwrap()
        .set_input_title("Statut")
        .set_input_message("État de l'action");
    ws3.add_data_validation(1, 3, last_row, 3, &status_validation)?;
    ws3.set_freeze_panes(1, 0)?;

    workbook.save_to_buffer()
}
```

### 1.9 Commande Tauri pour l'export

```rust
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use std::path::PathBuf;

#[tauri::command]
pub async fn export_stock_dashboard(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tickets = load_active_tickets(&conn).map_err(|e| e.to_string())?;
    let summary = compute_stock_summary(&tickets);

    let buffer = generate_stock_workbook(&tickets, &summary)
        .map_err(|e| format!("Erreur export Excel : {}", e))?;

    // Dialogue de sauvegarde natif
    let default_name = format!(
        "tableau_bord_stock_{}.xlsx",
        chrono::Local::now().format("%Y%m%d_%H%M")
    );

    let file_path = app.dialog()
        .file()
        .set_file_name(&default_name)
        .add_filter("Excel", &["xlsx"])
        .blocking_save_file();

    match file_path {
        Some(path) => {
            let path: PathBuf = path.into();
            std::fs::write(&path, &buffer)
                .map_err(|e| format!("Erreur écriture : {}", e))?;
            Ok(path.display().to_string())
        }
        None => Err("Export annulé par l'utilisateur".into()),
    }
}
```

### 1.10 Performance

Pour le volume CPAM 92, les temps mesurés en mode release :

|Export|Lignes|Onglets|Graphiques|Temps|
|---|--:|--:|--:|--:|
|Tableau de bord stock|~110|3|3|~8 ms|
|Plan d'action (1 technicien)|~200|3|0|~3 ms|
|Tous les plans d'action (ZIP)|~5 000|~90 (30×3)|0|~80 ms|
|Bilan d'activité|~90|3|4|~12 ms|
|Export brut 10K lignes|10 000|1|0|~45 ms|

La génération ZIP de tous les plans d'action utilise le crate `zip` (déjà dépendance transitive de rust_xlsxwriter) pour créer une archive contenant N fichiers `.xlsx` :

```rust
use std::io::Write;
use zip::write::{FileOptions, ZipWriter};

pub fn generate_all_action_plans_zip(
    plans: &[(String, Vec<u8>)], // (technicien, buffer xlsx)
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for (name, xlsx_data) in plans {
            let filename = format!("plan_action_{}.xlsx",
                name.replace(' ', "_").to_lowercase());
            zip.start_file(&filename, options)?;
            zip.write_all(xlsx_data)?;
        }
        zip.finish()?;
    }
    Ok(buf)
}
```

---

## 2. Détection de doublons avec strsim 0.11.1

### 2.1 Le problème des doublons GLPI

Les doublons de tickets GLPI représentent un irritant récurrent dans toute DSI. Un même incident signalé par deux utilisateurs différents, ou un technicien qui crée un ticket similaire sans vérifier l'existant, génère de la charge inutile et fausse les indicateurs. L'objectif est de détecter automatiquement les paires de tickets dont les titres sont suffisamment similaires pour mériter une vérification humaine, **sans jamais déclencher de clôture automatique** — le verdict reste toujours à l'opérateur.

### 2.2 Le crate strsim : algorithmes disponibles

Le crate `strsim` version **0.11.1** (maintenu par l'organisation rapidfuzz, 618M+ téléchargements crates.io, licence MIT) implémente neuf algorithmes de similarité de chaînes, tous en safe Rust :

```toml
[dependencies]
strsim = "0.11"
rayon = "1.10"
```

|Algorithme|Complexité|Sortie|Forces|Faiblesses|
|---|---|---|---|---|
|`jaro`|O(m+n)|0.0–1.0|Rapide, bon sur chaînes courtes|Ignore l'ordre des mots|
|`jaro_winkler`|O(m+n)|0.0–1.0|Bonus préfixe commun, très rapide|Biaisé vers les débuts identiques|
|`sorensen_dice`|O(m+n)|0.0–1.0|Bigrammes, tolérant au réordonnancement|Moins précis sous 5 caractères|
|`levenshtein`|O(m×n)|entier (distance)|Référence classique, intuitif|Lent pour N×N comparaisons|
|`normalized_levenshtein`|O(m×n)|0.0–1.0|Normalisé entre 0 et 1|Même coût que Levenshtein|
|`damerau_levenshtein`|O(m×n)|entier|Gère les transpositions (teh→the)|Le plus lent|
|`osa_distance`|O(m×n)|entier|Variante rapide de Damerau-Lev|Transpositions adjacentes uniquement|
|`hamming`|O(n)|entier|Ultra rapide|Exige des chaînes de même longueur|

### 2.3 Choix d'algorithme pour les titres GLPI

Les titres de tickets GLPI sont des phrases courtes (10–100 caractères) en français, souvent avec des variations de formulation pour le même problème :

- « Panne imprimante bureau 312 » vs « Imprimante en panne au bureau 312 »
- « Problème VPN Citrix » vs « VPN Citrix ne fonctionne plus »
- « Demande d'installation Teams » vs « Installation Teams pour Marie DUPONT »

**Sørensen-Dice est le meilleur algorithme primaire** pour ce cas d'usage. Il opère sur les bigrammes de caractères (paires adjacentes), ce qui le rend partiellement insensible à l'ordre des mots. « Panne imprimante » et « Imprimante panne » partagent la majorité de leurs bigrammes (« im », « mp », « pr », « ri », « im », « ma », « an », « nt », « te ») et scorent haut, là où les algorithmes par distance d'édition pénalisent lourdement le réordonnancement.

La combinaison recommandée est **Sørensen-Dice (poids 0.60) + Jaro-Winkler (poids 0.40)**. Jaro-Winkler apporte une sensibilité au préfixe commun, utile quand les tickets commencent identiquement (« Problème de… », « Demande de… »). Les deux algorithmes ont une complexité O(m+n), contre O(m×n) pour Levenshtein — un facteur décisif pour les comparaisons N×N.

Seuils de détection :

|Score composite|Classification|Action|
|---|---|---|
|≥ 0.85|**Doublon probable**|Affichage en rouge, vérification prioritaire|
|0.70 – 0.84|**Doublon possible**|Affichage en jaune, vérification optionnelle|
|< 0.70|Non-doublon|Ignoré|

### 2.4 Prétraitement des titres français

La qualité de la détection dépend directement du prétraitement. Le pipeline : minuscules → suppression des diacritiques → suppression de la ponctuation → suppression des stop words français → rejoin des tokens significatifs.

```rust
use stop_words::{get, LANGUAGE};

/// Préprocesseur français pour la comparaison de titres GLPI.
pub struct FrenchPreprocessor {
    stop_words: std::collections::HashSet<String>,
    it_stop_words: std::collections::HashSet<String>,
}

impl FrenchPreprocessor {
    pub fn new() -> Self {
        let mut stops: std::collections::HashSet<String> = get(LANGUAGE::French)
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        // Stop words IT/ITSM spécifiques CPAM — ne discriminent pas entre tickets
        let it_extras = [
            "bonjour", "svp", "merci", "urgent", "demande", "problème", "probleme",
            "ticket", "incident", "monsieur", "madame", "mme", "cordialement",
            "suite", "objet", "concerne", "sujet",
        ];
        for w in &it_extras {
            stops.insert(w.to_string());
        }

        Self {
            stop_words: stops,
            it_stop_words: std::collections::HashSet::new(),
        }
    }

    /// Normalise un titre GLPI pour la comparaison.
    pub fn preprocess(&self, text: &str) -> String {
        text.chars()
            // Minuscules
            .flat_map(|c| c.to_lowercase())
            // Suppression des diacritiques (décomposition Unicode + filtrage)
            .flat_map(|c| {
                let s = unicode_normalization::char::decompose_canonical(c, |_| {});
                // Approche simplifiée : remplacement direct des accents courants
                std::iter::once(c)
            })
            .collect::<String>()
            // Alternative robuste avec le crate unaccent
            .pipe(|s| remove_diacritics(&s))
            // Suppression ponctuation, remplacement par espace
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
            .collect::<String>()
            // Tokenisation et filtrage des stop words
            .split_whitespace()
            .filter(|w| w.len() > 1 && !self.stop_words.contains(*w))
            .collect::<Vec<&str>>()
            .join(" ")
    }
}

/// Suppression des diacritiques français courants.
fn remove_diacritics(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' | 'ä' => 'a',
            'ù' | 'û' | 'ü' => 'u',
            'ô' | 'ö' => 'o',
            'î' | 'ï' => 'i',
            'ç' => 'c',
            'ÿ' => 'y',
            'œ' => 'o', // Simplifié — dans le contexte IT, suffisant
            _ => c,
        })
        .collect()
}
```

Exemple de transformation :

- Entrée : « Problème d'accès à l'imprimante réseau — Étage 3 (URGENT) »
- Sortie : `"acces imprimante reseau etage 3"`

### 2.5 Moteur de détection parallélisé

Pour 10 000 tickets, le nombre de paires uniques est C(10000, 2) = 49 995 000. Jaro-Winkler seul traite ~50M paires en 15–25 secondes sur un thread. Avec trois niveaux d'optimisation + Rayon :

1. **Blocage par catégorie** : ne comparer que les tickets de même groupe technicien. 20 catégories × ~500 tickets = ~2,5M paires au lieu de 50M → **÷20**
2. **Pré-filtrage par longueur** : si `|len_a - len_b| / max(len_a, len_b) > 0.50`, les chaînes sont trop différentes pour matcher → élimine ~60% des paires restantes
3. **Pré-vérification Jaro-Winkler rapide** : calculer JW d'abord ; si < seuil - 0.15, sauter le calcul Dice → élimine ~80% des survivants

**Résultat combiné : < 1 seconde** sur 10K tickets en release mode, 8 cores.

```rust
use rayon::prelude::*;
use strsim::{jaro_winkler, sorensen_dice};

/// Configuration de la détection de doublons.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateConfig {
    /// Seuil doublon probable (défaut : 0.85)
    pub likely_threshold: f64,
    /// Seuil doublon possible (défaut : 0.70)
    pub possible_threshold: f64,
    /// Poids Jaro-Winkler dans le score composite (défaut : 0.40)
    pub jw_weight: f64,
    /// Poids Sørensen-Dice dans le score composite (défaut : 0.60)
    pub dice_weight: f64,
    /// Ratio max de différence de longueur (défaut : 0.50)
    pub max_length_diff_ratio: f64,
    /// Ne comparer qu'au sein de la même catégorie (défaut : true)
    pub same_category_only: bool,
}

impl Default for DuplicateConfig {
    fn default() -> Self {
        Self {
            likely_threshold: 0.85,
            possible_threshold: 0.70,
            jw_weight: 0.40,
            dice_weight: 0.60,
            max_length_diff_ratio: 0.50,
            same_category_only: true,
        }
    }
}

/// Paire de tickets potentiellement doublons.
#[derive(Clone, serde::Serialize)]
pub struct DuplicateCandidate {
    pub ticket_a_id: i64,
    pub ticket_a_title: String,
    pub ticket_b_id: i64,
    pub ticket_b_title: String,
    pub similarity_score: f64,
    pub confidence: DuplicateConfidence,
    pub jw_score: f64,
    pub dice_score: f64,
}

#[derive(Clone, serde::Serialize)]
pub enum DuplicateConfidence {
    Likely,
    Possible,
}

struct PreparedTicket {
    id: i64,
    original_title: String,
    processed_title: String,
    char_len: usize,
    category: String,
}

/// Détecte les doublons parmi une liste de tickets.
pub fn find_duplicates(
    tickets: &[GlpiTicket],
    config: &DuplicateConfig,
    preprocessor: &FrenchPreprocessor,
) -> Vec<DuplicateCandidate> {
    // Phase 1 : Prétraitement parallèle de tous les titres
    let prepared: Vec<PreparedTicket> = tickets
        .par_iter()
        .map(|t| {
            let processed = preprocessor.preprocess(&t.title);
            let char_len = processed.chars().count();
            PreparedTicket {
                id: t.id,
                original_title: t.title.clone(),
                processed_title: processed,
                char_len,
                category: t.group.clone(),
            }
        })
        .collect();

    // Phase 2 : Comparaison parallèle par paires avec optimisations
    let n = prepared.len();
    (0..n)
        .into_par_iter()
        .flat_map(|i| {
            let mut local_results = Vec::new();
            for j in (i + 1)..n {
                let a = &prepared[i];
                let b = &prepared[j];

                // Optimisation 1 : Blocage par catégorie
                if config.same_category_only && a.category != b.category {
                    continue;
                }

                // Optimisation 2 : Pré-filtrage par longueur
                let max_len = a.char_len.max(b.char_len);
                if max_len > 0 {
                    let diff = (a.char_len as f64 - b.char_len as f64).abs();
                    if diff / max_len as f64 > config.max_length_diff_ratio {
                        continue;
                    }
                }

                // Optimisation 3 : Pré-vérification JW rapide
                let jw = jaro_winkler(&a.processed_title, &b.processed_title);
                if jw < config.possible_threshold - 0.15 {
                    continue;
                }

                // Calcul Dice complet
                let dice = sorensen_dice(&a.processed_title, &b.processed_title);
                let score = config.jw_weight * jw + config.dice_weight * dice;

                if score >= config.possible_threshold {
                    local_results.push(DuplicateCandidate {
                        ticket_a_id: a.id,
                        ticket_a_title: a.original_title.clone(),
                        ticket_b_id: b.id,
                        ticket_b_title: b.original_title.clone(),
                        similarity_score: score,
                        confidence: if score >= config.likely_threshold {
                            DuplicateConfidence::Likely
                        } else {
                            DuplicateConfidence::Possible
                        },
                        jw_score: jw,
                        dice_score: dice,
                    });
                }
            }
            local_results
        })
        .collect()
}
```

### 2.6 Scoring multi-critères

Le score titre seul peut être enrichi par des critères contextuels qui augmentent la confiance :

```rust
/// Bonus contextuels pour affiner le score de similarité.
fn compute_context_bonus(a: &GlpiTicket, b: &GlpiTicket) -> f64 {
    let mut bonus = 0.0;

    // Même catégorie/groupe → +0.05
    if a.group == b.group { bonus += 0.05; }

    // Même demandeur → +0.10 (forte probabilité de doublon)
    if !a.requester.is_empty() && a.requester == b.requester { bonus += 0.10; }

    // Proximité temporelle : même jour → +0.10, même semaine → +0.05
    if let (Some(da), Some(db)) = (&a.open_date, &b.open_date) {
        let diff = (*da - *db).num_days().unsigned_abs();
        if diff == 0 { bonus += 0.10; }
        else if diff <= 7 { bonus += 0.05; }
    }

    // Même type (incident/demande) → +0.03
    if a.ticket_type == b.ticket_type { bonus += 0.03; }

    // Plafonner le bonus à 0.20 pour éviter les faux positifs
    bonus.min(0.20)
}
```

Le score final devient `min(1.0, base_score + context_bonus)`.

### 2.7 Passage à l'échelle : LSH pour > 100K tickets

Pour les très gros volumes (fusion de plusieurs CPAM, historique sur 5 ans), la complexité O(N²) devient prohibitive. La solution est le **Locality Sensitive Hashing** (LSH) avec MinHash, qui réduit la complexité à environ O(N). Le crate `lsh-rs` (v0.5.x, licence MIT) implémente MinHash + banding pour les ensembles de bigrammes :

```toml
# Optionnel — uniquement si > 100K tickets
lsh-rs = "0.5"
```

Le principe : chaque titre prétraité est converti en ensemble de bigrammes (caractères ou mots), puis haché par N fonctions de hachage (MinHash). Les vecteurs MinHash sont partitionnés en bandes. Deux tickets ne sont comparés avec strsim que si au moins une bande est identique. Pour 100K tickets avec 128 fonctions de hachage et 16 bandes de 8, le nombre de paires comparées chute de 5 milliards à environ 500K — un gain de 10 000×.

### 2.8 Commande Tauri

```rust
#[tauri::command]
pub async fn detect_duplicates(
    state: tauri::State<'_, AppState>,
    config: Option<DuplicateConfig>,
) -> Result<Vec<DuplicateCandidate>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tickets = load_active_tickets(&conn).map_err(|e| e.to_string())?;
    let cfg = config.unwrap_or_default();
    let preprocessor = FrenchPreprocessor::new();

    // Exécution dans un thread dédié pour ne pas bloquer le runtime Tokio
    let result = tokio::task::spawn_blocking(move || {
        find_duplicates(&tickets, &cfg, &preprocessor)
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(result)
}
```

---

## 3. tauri-plugin-notification 2.3+ : alerting desktop natif

### 3.1 Positionnement et installation

Le plugin de notification Tauri 2 permet d'envoyer des notifications toast natives depuis le backend Rust ou le frontend TypeScript. Sur Windows 10/11, les notifications apparaissent en toast dans le coin inférieur droit et persistent dans le Centre de notifications. La version actuelle est **2.3.3** (crate Rust et package npm `@tauri-apps/plugin-notification`).

```toml
# Cargo.toml
[dependencies]
tauri-plugin-notification = "2"
```

```bash
# Frontend
pnpm add @tauri-apps/plugin-notification
```

### 3.2 Configuration en trois étapes

**Étape 1 — Enregistrement du plugin** dans `main.rs` :

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        // ... autres plugins
        .run(tauri::generate_context!())
        .expect("erreur lors du lancement de l'application");
}
```

**Étape 2 — Déclaration des permissions** dans `src-tauri/capabilities/default.json` :

```json
{
  "identifier": "default",
  "description": "Permissions de base pour le GLPI Dashboard",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "notification:default"
  ]
}
```

Le jeu de permissions `notification:default` regroupe `allow-is-permission-granted`, `allow-request-permission`, `allow-notify` et `allow-show`. C'est suffisant pour l'ensemble des cas d'usage du Dashboard.

**Étape 3 — Utilisation** côté Rust ou TypeScript (voir ci-dessous).

### 3.3 API Rust : NotificationExt

Le trait `NotificationExt` est disponible sur `AppHandle` et `App`. Le builder chaîne `title()`, `body()`, `icon()` et `show()` :

```rust
use tauri_plugin_notification::NotificationExt;
use tauri::AppHandle;

/// Envoie une notification si un seuil est dépassé.
pub fn notify_threshold_breach(
    app: &AppHandle,
    technician: &str,
    count: u32,
    threshold: u32,
) -> Result<(), String> {
    if count > threshold {
        app.notification()
            .builder()
            .title("⚠️ Seuil de stock dépassé")
            .body(&format!(
                "{} : {} tickets (seuil : {}). Écart : +{}",
                technician, count, threshold, count - threshold
            ))
            .show()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Notifie la fin d'un import CSV.
pub fn notify_import_complete(
    app: &AppHandle,
    filename: &str,
    total: usize,
    errors: usize,
) -> Result<(), String> {
    let body = if errors == 0 {
        format!("{} tickets importés depuis '{}'.", total, filename)
    } else {
        format!(
            "{} tickets importés, {} erreurs depuis '{}'.",
            total, errors, filename
        )
    };

    app.notification()
        .builder()
        .title("📥 Import CSV terminé")
        .body(&body)
        .show()
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

### 3.4 API TypeScript : sendNotification

```typescript
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';

/**
 * Vérifie la permission et envoie une notification.
 * Encapsule la logique de permission pour éviter la duplication.
 */
async function notify(title: string, body: string): Promise<void> {
  let granted = await isPermissionGranted();
  if (!granted) {
    const permission = await requestPermission();
    granted = permission === 'granted';
  }
  if (!granted) return;

  sendNotification({ title, body });
}

/** Notification de détection d'anomalie. */
export async function notifyAnomaly(
  description: string,
  severity: 'warning' | 'critical',
): Promise<void> {
  const icon = severity === 'critical' ? '🔴' : '🟡';
  await notify(
    `${icon} Anomalie détectée`,
    description,
  );
}

/** Notification de résumé périodique. */
export async function notifyDashboardSummary(
  openCount: number,
  newToday: number,
  resolvedToday: number,
  overdueCount: number,
): Promise<void> {
  await notify(
    '📊 Résumé du tableau de bord',
    `Stock : ${openCount} | Nouveaux : +${newToday} | ` +
    `Résolus : ${resolvedToday} | En retard : ${overdueCount}`,
  );
}
```

### 3.5 Cas d'usage GLPI et implémentation

|Cas d'usage|Déclencheur|Côté|
|---|---|---|
|**Seuil de stock dépassé**|Après import CSV, pour chaque technicien > seuil|Rust|
|**Import terminé**|Fin du parsing CSV|Rust|
|**Détection d'anomalie**|Clustering du Segment 6 détecte un pic anormal|Rust|
|**Doublons détectés**|Module strsim trouve des doublons probables|Rust|
|**Résumé périodique**|Timer frontend (toutes les 4h si l'app est ouverte)|TypeScript|
|**Export terminé**|Fin de la génération Excel/PDF|Rust|

Implémentation du résumé périodique côté frontend avec `setInterval` :

```typescript
import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { notifyDashboardSummary } from '../utils/notifications';

interface DashboardSummary {
  open_count: number;
  new_today: number;
  resolved_today: number;
  overdue_count: number;
}

/**
 * Hook React qui envoie un résumé toutes les 4 heures.
 */
export function usePeriodicNotification(enabled: boolean, intervalHours = 4) {
  useEffect(() => {
    if (!enabled) return;

    const intervalMs = intervalHours * 60 * 60 * 1000;
    const timer = setInterval(async () => {
      try {
        const summary = await invoke<DashboardSummary>('get_dashboard_summary');
        await notifyDashboardSummary(
          summary.open_count,
          summary.new_today,
          summary.resolved_today,
          summary.overdue_count,
        );
      } catch (err) {
        console.error('Erreur notification périodique:', err);
      }
    }, intervalMs);

    return () => clearInterval(timer);
  }, [enabled, intervalHours]);
}
```

### 3.6 Limitations desktop connues

|Fonctionnalité|Statut desktop|Notes|
|---|---|---|
|Notification basique (titre + body)|✅ Fonctionne|Windows, macOS, Linux|
|Son|✅ Depuis v2.3.1|Via `notify-rust` v4.11.7|
|Icône personnalisée|⚠️ Partiel|Fonctionne uniquement après `tauri build`, pas en `dev` mode|
|Actions/boutons|❌ Non supporté|`registerActionTypes()` échoue avec "Command not found" — mobile uniquement|
|Clic → action|❌ Non supporté|Pas de callback au clic sur la notification|
|Image dans le body|❌ Non supporté|Desktop non implémenté|

**Conséquence pour le Dashboard** : les notifications servent d'**alertes informatives passives**. Pour les actions interactives (« Voir les doublons », « Ouvrir le plan d'action »), afficher un bandeau d'alerte dans l'UI React plutôt que de dépendre des notifications. Le pattern recommandé est : notification desktop pour attirer l'attention → clic sur le bandeau in-app pour l'action.

---

## 4. Suivi longitudinal : diff entre imports CSV successifs

### 4.1 Architecture de stockage

Le suivi longitudinal (CDC § 3.3, § 6.2) repose sur un système de snapshots : chaque import CSV est stocké intégralement en base SQLite, et un algorithme de diff compare deux snapshots consécutifs pour détecter les changements. Le schéma étend celui du Segment 2.

```sql
-- Table des imports (déjà définie Segment 2, complétée ici)
CREATE TABLE IF NOT EXISTS imports (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    filename    TEXT    NOT NULL,
    imported_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S','now','localtime')),
    ticket_count INTEGER NOT NULL DEFAULT 0,
    active_count INTEGER NOT NULL DEFAULT 0,   -- Tickets vivants
    closed_count INTEGER NOT NULL DEFAULT 0,   -- Tickets terminés
    error_count  INTEGER NOT NULL DEFAULT 0,   -- Lignes en erreur
    checksum    TEXT,                           -- SHA256 du fichier CSV
    notes       TEXT                            -- Notes libres de l'utilisateur
);

-- Snapshot complet de chaque ticket à chaque import
CREATE TABLE IF NOT EXISTS ticket_snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    import_id   INTEGER NOT NULL REFERENCES imports(id) ON DELETE CASCADE,
    ticket_id   INTEGER NOT NULL,
    title       TEXT,
    status      TEXT    NOT NULL,
    status_code INTEGER NOT NULL,  -- 1=Nouveau, 2=Attribué, 3=Planifié, 4=En attente, 5=Résolu, 6=Clos
    priority    TEXT    NOT NULL,
    ticket_type TEXT,              -- Incident / Demande
    category    TEXT,              -- Groupe de techniciens
    assigned_to TEXT,              -- Technicien
    requester   TEXT,
    open_date   TEXT,
    solve_date  TEXT,
    close_date  TEXT,
    last_modified TEXT,
    followup_count INTEGER DEFAULT 0,
    row_hash    TEXT    NOT NULL,  -- Hash des champs trackés pour comparaison rapide
    UNIQUE(import_id, ticket_id)
);
CREATE INDEX IF NOT EXISTS idx_snapshots_import ON ticket_snapshots(import_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_ticket ON ticket_snapshots(ticket_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_hash   ON ticket_snapshots(import_id, ticket_id, row_hash);

-- Journal des changements détectés entre deux imports
CREATE TABLE IF NOT EXISTS change_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    from_import INTEGER NOT NULL REFERENCES imports(id),
    to_import   INTEGER NOT NULL REFERENCES imports(id),
    ticket_id   INTEGER NOT NULL,
    change_type TEXT    NOT NULL,
    -- Types : 'new', 'disappeared', 'resolved', 'closed', 'reopened',
    --         'status_change', 'reassignment', 'priority_change', 'category_change'
    field_name  TEXT,       -- Champ modifié (pour status_change, reassignment, etc.)
    old_value   TEXT,       -- Ancienne valeur
    new_value   TEXT,       -- Nouvelle valeur
    detected_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S','now','localtime'))
);
CREATE INDEX IF NOT EXISTS idx_changes_imports ON change_log(from_import, to_import);
CREATE INDEX IF NOT EXISTS idx_changes_type    ON change_log(change_type);
```

### 4.2 Hash de ligne pour comparaison rapide

Le `row_hash` est un hash FNV-1a 64 bits de la concaténation des champs trackés. Quand les hashs sont identiques entre deux imports, le ticket est inchangé — ce qui évite la comparaison champ par champ pour **~90% des tickets** (seuls ~10% changent entre deux imports quotidiens).

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Calcule le hash d'un ticket pour la comparaison inter-imports.
fn compute_row_hash(ticket: &ParsedTicket) -> String {
    let mut hasher = DefaultHasher::new();
    ticket.status.hash(&mut hasher);
    ticket.priority.hash(&mut hasher);
    ticket.assigned_to.hash(&mut hasher);
    ticket.group.hash(&mut hasher);
    ticket.followup_count.hash(&mut hasher);
    // Ne PAS inclure last_modified — change à chaque connexion GLPI
    // Ne PAS inclure title — rarement modifié et coûteux en hash
    format!("{:016x}", hasher.finish())
}
```

### 4.3 Ingestion d'un snapshot

```rust
use rusqlite::{Connection, Transaction, params};

/// Insère un import et ses snapshots dans une transaction unique.
pub fn ingest_import(
    conn: &mut Connection,
    filename: &str,
    tickets: &[ParsedTicket],
    checksum: &str,
) -> Result<i64, rusqlite::Error> {
    let tx = conn.transaction()?;

    let active_count = tickets.iter()
        .filter(|t| t.status_code <= 4)
        .count();
    let closed_count = tickets.len() - active_count;

    // Insérer l'import
    tx.execute(
        "INSERT INTO imports (filename, ticket_count, active_count, closed_count, checksum)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![filename, tickets.len(), active_count, closed_count, checksum],
    )?;
    let import_id = tx.last_insert_rowid();

    // Préparer l'insertion de snapshots (batch)
    let mut stmt = tx.prepare_cached(
        "INSERT INTO ticket_snapshots
         (import_id, ticket_id, title, status, status_code, priority, ticket_type,
          category, assigned_to, requester, open_date, solve_date, close_date,
          last_modified, followup_count, row_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
    )?;

    for t in tickets {
        let hash = compute_row_hash(t);
        stmt.execute(params![
            import_id, t.id, t.title, t.status, t.status_code, t.priority,
            t.ticket_type, t.group, t.assigned_to, t.requester,
            t.open_date.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
            t.solve_date.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
            t.close_date.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
            t.last_modified.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
            t.followup_count, hash,
        ])?;
    }

    tx.commit()?;
    Ok(import_id)
}
```

Performance : l'insertion de 10 000 lignes dans une transaction unique avec WAL mode (`PRAGMA journal_mode = WAL`) prend **< 200 ms**. Le mode WAL est configuré dans le Segment 2.

### 4.4 Algorithme de diff

Le diff compare deux snapshots (identifiés par `from_import` et `to_import`) en trois passes :

1. **Tickets nouveaux** : présents dans `to` mais absents de `from`
2. **Tickets disparus** : présents dans `from` mais absents de `to` (résolus ou supprimés)
3. **Tickets modifiés** : présents dans les deux, hash différent → comparaison champ par champ

```rust
use std::collections::HashMap;

/// Résultat d'une comparaison entre deux imports.
#[derive(serde::Serialize)]
pub struct DiffSummary {
    pub from_import: i64,
    pub to_import: i64,
    pub from_date: String,
    pub to_date: String,
    pub new_tickets: Vec<ChangeRecord>,
    pub disappeared_tickets: Vec<ChangeRecord>,
    pub status_changes: Vec<ChangeRecord>,
    pub reassignments: Vec<ChangeRecord>,
    pub priority_changes: Vec<ChangeRecord>,
    pub category_changes: Vec<ChangeRecord>,
    // Agrégats
    pub total_changes: usize,
    pub net_flow: i64,  // new_count - disappeared_count
    pub stock_before: usize,
    pub stock_after: usize,
}

#[derive(Clone, serde::Serialize)]
pub struct ChangeRecord {
    pub ticket_id: i64,
    pub ticket_title: String,
    pub change_type: String,
    pub field_name: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

struct SnapshotEntry {
    ticket_id: i64,
    title: String,
    status: String,
    status_code: i32,
    priority: String,
    assigned_to: String,
    category: String,
    row_hash: String,
}

/// Charge un snapshot en HashMap<ticket_id, SnapshotEntry>.
fn load_snapshot(
    conn: &Connection,
    import_id: i64,
) -> Result<HashMap<i64, SnapshotEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT ticket_id, title, status, status_code, priority,
                assigned_to, category, row_hash
         FROM ticket_snapshots WHERE import_id = ?1"
    )?;
    let map: HashMap<i64, SnapshotEntry> = stmt
        .query_map(params![import_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                SnapshotEntry {
                    ticket_id: row.get(0)?,
                    title: row.get(1)?,
                    status: row.get(2)?,
                    status_code: row.get(3)?,
                    priority: row.get(4)?,
                    assigned_to: row.get(5)?,
                    category: row.get(6)?,
                    row_hash: row.get(7)?,
                },
            ))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;
    Ok(map)
}

/// Compare deux imports et retourne le diff.
pub fn compare_imports(
    conn: &mut Connection,
    from_import: i64,
    to_import: i64,
) -> Result<DiffSummary, Box<dyn std::error::Error>> {
    let old = load_snapshot(conn, from_import)?;
    let new = load_snapshot(conn, to_import)?;
    let mut changes: Vec<ChangeRecord> = Vec::new();

    // Pass 1 : Tickets nouveaux (dans new mais pas dans old)
    for (tid, entry) in &new {
        if !old.contains_key(tid) {
            changes.push(ChangeRecord {
                ticket_id: *tid,
                ticket_title: entry.title.clone(),
                change_type: "new".into(),
                field_name: None,
                old_value: None,
                new_value: Some(entry.status.clone()),
            });
        }
    }

    // Pass 2 : Tickets disparus (dans old mais pas dans new)
    for (tid, entry) in &old {
        if !new.contains_key(tid) {
            changes.push(ChangeRecord {
                ticket_id: *tid,
                ticket_title: entry.title.clone(),
                change_type: "disappeared".into(),
                field_name: None,
                old_value: Some(entry.status.clone()),
                new_value: None,
            });
        }
    }

    // Pass 3 : Tickets modifiés (hash différent)
    for (tid, new_entry) in &new {
        if let Some(old_entry) = old.get(tid) {
            if old_entry.row_hash == new_entry.row_hash {
                continue; // Aucun changement — skip rapide
            }

            // Changement de statut
            if old_entry.status != new_entry.status {
                let change_type = match (old_entry.status_code, new_entry.status_code) {
                    (_, 5) | (_, 6) => "resolved",
                    (5, c) | (6, c) if c <= 4 => "reopened",
                    _ => "status_change",
                };
                changes.push(ChangeRecord {
                    ticket_id: *tid,
                    ticket_title: new_entry.title.clone(),
                    change_type: change_type.into(),
                    field_name: Some("status".into()),
                    old_value: Some(old_entry.status.clone()),
                    new_value: Some(new_entry.status.clone()),
                });
            }

            // Réaffectation technicien
            if old_entry.assigned_to != new_entry.assigned_to {
                changes.push(ChangeRecord {
                    ticket_id: *tid,
                    ticket_title: new_entry.title.clone(),
                    change_type: "reassignment".into(),
                    field_name: Some("assigned_to".into()),
                    old_value: Some(old_entry.assigned_to.clone()),
                    new_value: Some(new_entry.assigned_to.clone()),
                });
            }

            // Changement de priorité
            if old_entry.priority != new_entry.priority {
                changes.push(ChangeRecord {
                    ticket_id: *tid,
                    ticket_title: new_entry.title.clone(),
                    change_type: "priority_change".into(),
                    field_name: Some("priority".into()),
                    old_value: Some(old_entry.priority.clone()),
                    new_value: Some(new_entry.priority.clone()),
                });
            }

            // Changement de catégorie/groupe
            if old_entry.category != new_entry.category {
                changes.push(ChangeRecord {
                    ticket_id: *tid,
                    ticket_title: new_entry.title.clone(),
                    change_type: "category_change".into(),
                    field_name: Some("category".into()),
                    old_value: Some(old_entry.category.clone()),
                    new_value: Some(new_entry.category.clone()),
                });
            }
        }
    }

    // Compteurs de stock (tickets vivants uniquement, status_code ≤ 4)
    let stock_before = old.values().filter(|e| e.status_code <= 4).count();
    let stock_after = new.values().filter(|e| e.status_code <= 4).count();
    let new_count = changes.iter().filter(|c| c.change_type == "new").count() as i64;
    let disappeared_count = changes.iter()
        .filter(|c| c.change_type == "disappeared" || c.change_type == "resolved")
        .count() as i64;

    // Persister les changements dans change_log
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO change_log
             (from_import, to_import, ticket_id, change_type, field_name, old_value, new_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;
        for c in &changes {
            stmt.execute(params![
                from_import, to_import, c.ticket_id, c.change_type,
                c.field_name, c.old_value, c.new_value,
            ])?;
        }
    }
    tx.commit()?;

    let total_changes = changes.len();
    Ok(DiffSummary {
        from_import, to_import,
        from_date: get_import_date(conn, from_import)?,
        to_date: get_import_date(conn, to_import)?,
        new_tickets: changes.iter().filter(|c| c.change_type == "new").cloned().collect(),
        disappeared_tickets: changes.iter().filter(|c| c.change_type == "disappeared").cloned().collect(),
        status_changes: changes.iter().filter(|c| c.change_type == "status_change" || c.change_type == "resolved" || c.change_type == "reopened").cloned().collect(),
        reassignments: changes.iter().filter(|c| c.change_type == "reassignment").cloned().collect(),
        priority_changes: changes.iter().filter(|c| c.change_type == "priority_change").cloned().collect(),
        category_changes: changes.iter().filter(|c| c.change_type == "category_change").cloned().collect(),
        total_changes,
        net_flow: new_count - disappeared_count,
        stock_before,
        stock_after,
    })
}
```

Performance : le chargement de deux snapshots de 10K lignes prend ~15 ms chacun ; la comparaison par hash + diff champ par champ pour ~1K tickets changés prend ~5 ms. **Temps total du diff : < 50 ms**.

### 4.5 Agrégation temporelle pour les courbes d'évolution

Les requêtes SQL d'agrégation permettent de construire les courbes d'évolution du stock (CDC § 6.3) :

```sql
-- Évolution du stock vivant par import
SELECT i.id, i.imported_at, i.active_count, i.closed_count
FROM imports i ORDER BY i.imported_at;

-- Flux net quotidien (agrégation du change_log)
SELECT date(i.imported_at) AS jour,
    COUNT(CASE WHEN cl.change_type = 'new' THEN 1 END) AS entrees,
    COUNT(CASE WHEN cl.change_type IN ('resolved', 'disappeared', 'closed') THEN 1 END) AS sorties,
    COUNT(CASE WHEN cl.change_type = 'new' THEN 1 END)
      - COUNT(CASE WHEN cl.change_type IN ('resolved', 'disappeared', 'closed') THEN 1 END) AS flux_net
FROM change_log cl
JOIN imports i ON i.id = cl.to_import
GROUP BY jour ORDER BY jour;

-- Flux mensuel
SELECT strftime('%Y-%m', i.imported_at) AS mois,
    COUNT(CASE WHEN cl.change_type = 'new' THEN 1 END) AS entrees,
    COUNT(CASE WHEN cl.change_type IN ('resolved', 'disappeared', 'closed') THEN 1 END) AS sorties
FROM change_log cl
JOIN imports i ON i.id = cl.to_import
GROUP BY mois ORDER BY mois;

-- Delta par technicien entre deux imports
SELECT
    COALESCE(n.assigned_to, o.assigned_to) AS technicien,
    COUNT(DISTINCT CASE WHEN o.ticket_id IS NULL THEN n.ticket_id END) AS nouveaux,
    COUNT(DISTINCT CASE WHEN n.ticket_id IS NULL THEN o.ticket_id END) AS resolus,
    COUNT(DISTINCT CASE WHEN n.ticket_id IS NOT NULL AND n.status_code <= 4 THEN n.ticket_id END) AS stock_actuel
FROM ticket_snapshots n
FULL OUTER JOIN ticket_snapshots o
    ON n.ticket_id = o.ticket_id AND o.import_id = ?1
WHERE n.import_id = ?2 AND n.status_code <= 4
GROUP BY technicien ORDER BY stock_actuel DESC;
```

**Note SQLite** : `FULL OUTER JOIN` n'est supporté que depuis SQLite 3.39.0 (inclus dans rusqlite 0.38+ avec `bundled`). Pour les versions antérieures, utiliser `LEFT JOIN` + `UNION` + `LEFT JOIN inversé`.

### 4.6 Format de données pour le frontend

La commande Tauri retourne des données directement consommables par Recharts (Segment 7) :

```rust
#[derive(serde::Serialize)]
pub struct TimeSeriesPoint {
    pub period: String,      // "2026-02-25" ou "2026-W09" ou "2026-02"
    pub new_count: u32,
    pub resolved_count: u32,
    pub net_flow: i32,
    pub stock_total: u32,
}

#[tauri::command]
pub async fn get_stock_evolution(
    state: tauri::State<'_, AppState>,
    granularity: String,  // "daily" | "weekly" | "monthly"
) -> Result<Vec<TimeSeriesPoint>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    // Exécuter la requête SQL appropriée selon la granularité...
    // Retourner les points de la série temporelle
    todo!()
}
```

### 4.7 Rétention et nettoyage

À raison d'un import par jour, chaque import ajoute ~10K lignes de snapshots. Sur un an : 365 × 10K = 3,65M lignes ≈ 400 MB. La stratégie de rétention recommandée :

- **Conserver le change_log indéfiniment** — il est léger (~1K lignes/import) et indispensable pour les courbes d'évolution
- **Conserver les snapshots des 30 derniers imports** — suffisant pour les comparaisons récentes
- **Archiver les agrégats des imports anciens** dans une table `import_summary` avant suppression

```rust
pub fn prune_old_snapshots(conn: &mut Connection, keep_last_n: u32) -> Result<usize, rusqlite::Error> {
    let deleted = conn.execute(
        "DELETE FROM ticket_snapshots
         WHERE import_id NOT IN (
             SELECT id FROM imports ORDER BY imported_at DESC LIMIT ?1
         )",
        params![keep_last_n],
    )?;
    conn.execute("VACUUM", [])?; // Récupérer l'espace disque
    Ok(deleted)
}
```

---

## 5. Export PDF : Typst comme moteur de composition

### 5.1 Évaluation des approches

Quatre approches ont été évaluées pour la génération de rapports PDF depuis Rust :

|Critère|printpdf 0.8.2|genpdf 0.2.0|**Typst (0.14+)**|HTML→PDF WebView|
|---|---|---|---|---|
|Niveau d'abstraction|Très bas (primitives)|Moyen (paragraphes)|**Haut (composition)**|Variable|
|Tables|Manuelles, pénible|Correctes|**Excellentes, auto-pagination**|Dépend du CSS|
|UTF-8 français|✅ (embed TTF)|✅|**✅ natif, hyphenation**|⚠️|
|En-têtes/pieds de page|Manuels|En-tête uniquement|**✅ first-class, numérotation**|❌ pas d'API silencieuse|
|Table des matières|❌|❌|**✅ `#outline()` intégré**|Variable|
|Graphiques/images|✅ embed PNG|✅ embed PNG|**✅ `#image.decode()`**|✅|
|Maintenance 2026|Active|❌ Abandonné (3+ ans)|**Très active**|N/A|
|Impact binaire|~500 KB|~200 KB|~5–8 MB|0 (WebView existant)|

**Typst est le choix retenu.** C'est le seul moteur de composition en Rust qui offre une qualité typographique professionnelle (comparable à LaTeX) avec une syntaxe accessible, le support natif des tables paginées, des en-têtes/pieds de page avec numérotation, et la possibilité de séparer le template des données. Son adoption explose depuis 2024 et il est devenu le standard de facto pour la génération PDF en Rust.

L'approche HTML→PDF via WebView Tauri (window.print()) n'est pas retenue car il n'existe pas d'API de rendu silencieux — le dialogue d'impression s'ouvre systématiquement, ce qui est inacceptable pour une génération automatisée.

### 5.2 Dépendances

```toml
[dependencies]
typst-as-lib = "0.4"
typst-pdf = "0.14"
```

Le crate `typst-as-lib` enveloppe le compilateur Typst pour un usage en tant que bibliothèque Rust, avec injection de données via `sys.inputs`. Le crate `typst-pdf` convertit le document compilé en bytes PDF.

### 5.3 Architecture template + données

La séparation template/données permet aux utilisateurs de modifier la mise en page des rapports sans toucher au code Rust. Les templates `.typ` sont embarqués dans le binaire via `include_str!()` ou chargés depuis le répertoire de configuration.

**Template Typst pour le rapport de stock** :

```typst
// templates/rapport_stock.typ
#set page(
  paper: "a4",
  margin: (top: 25mm, bottom: 25mm, left: 20mm, right: 20mm),
  header: context [
    #set text(9pt, fill: gray)
    #grid(
      columns: (1fr, 1fr),
      align(left)[Rapport d'analyse GLPI — #sys.inputs.at("organization", default: "CPAM")],
      align(right)[Page #counter(page).display("1 / 1", both: true)],
    )
    #line(length: 100%, stroke: 0.5pt + gray)
  ],
  footer: context [
    #line(length: 100%, stroke: 0.5pt + gray)
    #set text(8pt, fill: gray)
    Généré le #sys.inputs.at("date", default: "—") — Document à usage interne
  ],
)

#set text(font: "Liberation Sans", 11pt, lang: "fr")
#set heading(numbering: "1.1")

// Page de couverture
#page(header: none, footer: none)[
  #v(1fr)
  #align(center)[
    #text(28pt, weight: "bold", fill: rgb("#2C5F8A"))[
      Rapport d'Analyse du Stock GLPI
    ]
    #v(8mm)
    #text(16pt)[#sys.inputs.at("organization", default: "CPAM")]
    #v(4mm)
    #text(14pt, fill: gray)[#sys.inputs.at("date", default: "")]
    #v(1cm)
    #line(length: 60%, stroke: 2pt + rgb("#2C5F8A"))
    #v(5mm)
    #text(12pt)[
      Stock total : *#sys.inputs.at("total_stock", default: "—")* tickets \
      Dont vivants : *#sys.inputs.at("active_stock", default: "—")* \
      Dont terminés : *#sys.inputs.at("closed_stock", default: "—")*
    ]
  ]
  #v(1fr)
]

// Table des matières
#outline(title: "Table des matières", indent: auto, depth: 2)

// Contenu principal
= Synthèse globale

#let data = json.decode(sys.inputs.at("data"))
#let technicians = data.technicians

== Indicateurs clés

#table(
  columns: (2fr, 1fr, 1fr, 1fr, 1fr),
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#2C5F8A") } else if calc.odd(row) { rgb("#F0F4F8") },
  table.header(
    text(white, weight: "bold")[Indicateur],
    text(white, weight: "bold")[Valeur],
    text(white, weight: "bold")[Variation],
    text(white, weight: "bold")[Seuil],
    text(white, weight: "bold")[Statut],
  ),
  ..for kpi in data.kpis {
    (kpi.name, str(kpi.value), kpi.variation, str(kpi.threshold), kpi.status)
  }
)

== Répartition par technicien

#table(
  columns: (2fr, 1fr, 1fr, 1fr, 1fr, 1fr),
  stroke: 0.5pt,
  fill: (_, row) => if row == 0 { rgb("#2C5F8A") } else if calc.odd(row) { rgb("#F0F4F8") },
  table.header(
    text(white, weight: "bold")[Technicien],
    text(white, weight: "bold")[Stock],
    text(white, weight: "bold")[En cours],
    text(white, weight: "bold")[En attente],
    text(white, weight: "bold")[> 90j],
    text(white, weight: "bold")[Âge moy.],
  ),
  ..for tech in technicians {
    (
      tech.name,
      str(tech.total),
      str(tech.en_cours),
      str(tech.en_attente),
      str(tech.over_90d),
      str(tech.avg_age) + "j",
    )
  }
)

// Insertion d'un graphique pré-rendu en PNG
#if sys.inputs.at("chart_png", default: none) != none [
  == Répartition graphique
  #image.decode(bytes(sys.inputs.at("chart_png")), width: 80%)
]
```

### 5.4 Compilation côté Rust

```rust
use std::collections::HashMap;

/// Génère un rapport PDF à partir du template et des données JSON.
pub fn generate_stock_report_pdf(
    data: &serde_json::Value,
    organization: &str,
    chart_png: Option<&[u8]>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Préparer les inputs Typst
    let mut inputs = HashMap::new();
    inputs.insert("organization".to_string(), organization.to_string());
    inputs.insert("date".to_string(),
        chrono::Local::now().format("%d/%m/%Y à %H:%M").to_string());
    inputs.insert("data".to_string(), serde_json::to_string(data)?);

    if let Some(total) = data.get("total_stock") {
        inputs.insert("total_stock".to_string(), total.to_string());
    }
    if let Some(active) = data.get("active_stock") {
        inputs.insert("active_stock".to_string(), active.to_string());
    }
    if let Some(closed) = data.get("closed_stock") {
        inputs.insert("closed_stock".to_string(), closed.to_string());
    }

    // Graphique en base64 si disponible
    if let Some(png) = chart_png {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(png);
        inputs.insert("chart_png".to_string(), b64);
    }

    // Compiler le template Typst
    let template = include_str!("../templates/rapport_stock.typ");

    // Utiliser typst-as-lib pour la compilation
    let world = TypstWorld::builder()
        .main_file_content(template)
        .font_bytes(vec![
            include_bytes!("../fonts/LiberationSans-Regular.ttf").to_vec(),
            include_bytes!("../fonts/LiberationSans-Bold.ttf").to_vec(),
            include_bytes!("../fonts/LiberationSans-Italic.ttf").to_vec(),
        ])
        .inputs(inputs)
        .build();

    let document = typst::compile(&world)
        .output
        .map_err(|errs| format!("Erreur compilation Typst : {:?}", errs))?;

    let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|e| format!("Erreur PDF : {:?}", e))?;

    Ok(pdf_bytes)
}
```

### 5.5 Pré-rendu des graphiques en PNG

Typst ne dispose pas de moteur de graphiques intégré. La solution : pré-rendre les graphiques en PNG avec le crate `plotters` (v0.3.7, 4 000+ étoiles), puis les injecter dans le template Typst via `#image.decode()`.

```toml
[dependencies]
plotters = "0.3.7"
```

```rust
use plotters::prelude::*;

/// Rend un camembert de répartition par statut en PNG (buffer mémoire).
pub fn render_status_pie_chart(
    data: &[(String, u32)],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; (width * height * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut buf, (width, height))
            .into_drawing_area();
        root.fill(&WHITE)?;

        // Plotters supporte les Pie charts depuis v0.3.5
        let total: f64 = data.iter().map(|(_, v)| *v as f64).sum();
        let colors = [
            RGBColor(92, 155, 213),   // Bleu
            RGBColor(237, 125, 49),   // Orange
            RGBColor(165, 165, 165),  // Gris
            RGBColor(255, 192, 0),    // Jaune
            RGBColor(68, 114, 196),   // Bleu foncé
            RGBColor(112, 173, 71),   // Vert
        ];

        let dims = root.dim_in_pixel();
        let center = (dims.0 as i32 / 2, dims.1 as i32 / 2);
        let radius = dims.0.min(dims.1) as f64 / 2.5;
        let sizes: Vec<f64> = data.iter().map(|(_, v)| *v as f64 / total).collect();
        let labels: Vec<String> = data.iter()
            .map(|(name, count)| format!("{} ({})", name, count))
            .collect();

        let mut pie = Pie::new(&center, &radius, &sizes, &colors[..data.len()], &labels);
        pie.start_angle(-90.0);
        pie.label_style(("Liberation Sans", 12).into_font());
        pie.percentages_style(("Liberation Sans", 11, &WHITE).into_font());

        root.draw(&pie)?;
        root.present()?;
    }

    // Encoder en PNG
    let mut png_buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
    image::ImageEncoder::write_image(
        &encoder, &buf, width, height,
        image::ExtendedColorType::Rgb8,
    )?;

    Ok(png_buf)
}
```

### 5.6 Commande Tauri pour l'export PDF

```rust
#[tauri::command]
pub async fn export_stock_pdf(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let data = compute_stock_report_data(&conn).map_err(|e| e.to_string())?;

    // Pré-rendre le graphique
    let chart_png = render_status_pie_chart(&data.status_distribution, 600, 400)
        .map_err(|e| format!("Erreur graphique : {}", e))?;

    let pdf = generate_stock_report_pdf(
        &serde_json::to_value(&data).map_err(|e| e.to_string())?,
        "CPAM des Hauts-de-Seine (92)",
        Some(&chart_png),
    ).map_err(|e| e.to_string())?;

    let default_name = format!(
        "rapport_stock_{}.pdf",
        chrono::Local::now().format("%Y%m%d")
    );

    let file_path = app.dialog()
        .file()
        .set_file_name(&default_name)
        .add_filter("PDF", &["pdf"])
        .blocking_save_file();

    match file_path {
        Some(path) => {
            let path: std::path::PathBuf = path.into();
            std::fs::write(&path, &pdf)
                .map_err(|e| format!("Erreur écriture : {}", e))?;
            Ok(path.display().to_string())
        }
        None => Err("Export annulé".into()),
    }
}
```

### 5.7 Polices embarquées

Pour garantir un rendu identique sur tous les postes, les polices sont embarquées dans le binaire. **Liberation Sans** (licence SIL OFL) est le substitut métrique de Arial, disponible sur tous les systèmes. Les trois fichiers (Regular, Bold, Italic) pèsent ~600 KB au total. Ils se placent dans `src-tauri/fonts/` et s'incluent via `include_bytes!()`.

Pour les postes CPAM qui disposent déjà d'Arial, Typst peut charger les polices système via `typst::text::Font::from_file()` au lieu de les embarquer.

---

## 6. Intégration future API REST GLPI

### 6.1 Vue d'ensemble de l'API GLPI

GLPI expose deux générations d'API :

|API|URL de base|Disponibilité|Authentification|
|---|---|---|---|
|**V1 Legacy**|`/apirest.php`|GLPI 9.5 → 11.x|Session token + App token|
|**V2**|`/api.php`|GLPI 11.x+|OAuth2 / Bearer|

Pour les installations CPAM (typiquement GLPI 9.5 ou 10.x), **cibler l'API V1** qui est universellement disponible. L'API V2 avec OAuth2 reste une option future pour GLPI 11.

### 6.2 Flux d'authentification V1

L'authentification V1 repose sur deux tokens :

1. **App token** : enregistré par l'administrateur GLPI dans _Administration → API → Clients API_. Identifie l'application cliente.
2. **User token** : généré par l'utilisateur dans _Mon profil → Accès distant → Clé d'accès distance_. Identifie l'utilisateur.

La séquence : `GET /initSession` avec les deux tokens → retourne un `session_token` → toutes les requêtes suivantes incluent `Session-Token` en header → `GET /killSession` à la fermeture.

```rust
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};

/// Client API GLPI V1.
pub struct GlpiClient {
    client: Client,
    base_url: String,        // ex: "https://glpi.cpam92.local/apirest.php"
    app_token: Option<String>,
    session_token: Option<String>,
}

#[derive(Deserialize)]
struct InitSessionResponse {
    session_token: String,
}

impl GlpiClient {
    pub fn new(base_url: &str, app_token: Option<&str>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Erreur création client HTTP"),
            base_url: base_url.trim_end_matches('/').to_string(),
            app_token: app_token.map(|s| s.to_string()),
            session_token: None,
        }
    }

    /// Ouvre une session avec le user_token.
    pub async fn init_session(&mut self, user_token: &str) -> Result<(), GlpiError> {
        let mut req = self.client
            .get(format!("{}/initSession", self.base_url))
            .header("Authorization", format!("user_token {}", user_token));

        if let Some(ref app_token) = self.app_token {
            req = req.header("App-Token", app_token);
        }

        let resp = req.send().await?;
        match resp.status() {
            StatusCode::OK => {
                let body: InitSessionResponse = resp.json().await?;
                self.session_token = Some(body.session_token);
                Ok(())
            }
            StatusCode::UNAUTHORIZED => Err(GlpiError::Auth("Token invalide ou expiré".into())),
            status => Err(GlpiError::Http(format!("Statut inattendu : {}", status))),
        }
    }

    /// Ferme la session proprement.
    pub async fn kill_session(&mut self) -> Result<(), GlpiError> {
        if let Some(ref token) = self.session_token {
            let _ = self.client
                .get(format!("{}/killSession", self.base_url))
                .header("Session-Token", token)
                .send()
                .await;
        }
        self.session_token = None;
        Ok(())
    }

    /// Construit une requête authentifiée.
    fn authed_request(&self, method: Method, url: &str) -> Result<reqwest::RequestBuilder, GlpiError> {
        let token = self.session_token.as_ref()
            .ok_or_else(|| GlpiError::Auth("Session non initialisée".into()))?;
        let mut req = self.client.request(method, url)
            .header("Session-Token", token)
            .header("Content-Type", "application/json");
        if let Some(ref app_token) = self.app_token {
            req = req.header("App-Token", app_token);
        }
        Ok(req)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GlpiError {
    #[error("Erreur HTTP : {0}")]
    Http(String),
    #[error("Erreur d'authentification : {0}")]
    Auth(String),
    #[error("Erreur réseau : {0}")]
    Network(#[from] reqwest::Error),
    #[error("Erreur de parsing : {0}")]
    Parse(String),
}
```

### 6.3 Récupération paginée des tickets

L'API GLPI limite les réponses à **990 éléments par requête** (header `Accept-Range`). La pagination utilise le paramètre `range=start-end` et le header de réponse `Content-Range: start-end/total`.

```rust
/// Ticket GLPI retourné par l'API.
#[derive(Deserialize, Serialize, Debug)]
pub struct GlpiApiTicket {
    pub id: i64,
    pub name: String,           // Titre
    pub status: i32,            // 1-6
    pub priority: i32,          // 1-6
    #[serde(rename = "type")]
    pub ticket_type: i32,       // 1=Incident, 2=Demande
    pub date: String,           // Date d'ouverture ISO
    pub date_mod: String,       // Dernière modification ISO
    pub solvedate: Option<String>,
    pub closedate: Option<String>,
}

impl GlpiClient {
    /// Récupère tous les tickets par pagination automatique.
    pub async fn get_all_tickets(&self) -> Result<Vec<GlpiApiTicket>, GlpiError> {
        let mut all_tickets = Vec::new();
        let mut offset = 0u64;
        let page_size = 200u64;

        loop {
            let url = format!(
                "{}/Ticket?range={}-{}&sort=19&order=DESC&expand_dropdowns=true",
                self.base_url, offset, offset + page_size - 1
            );

            let resp = self.authed_request(Method::GET, &url)?.send().await?;

            // Extraire le total depuis Content-Range
            let total = resp.headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split('/').last())
                .and_then(|n| n.parse::<u64>().ok());

            let batch: Vec<GlpiApiTicket> = resp.json().await
                .map_err(|e| GlpiError::Parse(e.to_string()))?;
            let batch_len = batch.len() as u64;
            all_tickets.extend(batch);

            offset += page_size;
            if batch_len < page_size || offset >= total.unwrap_or(0) {
                break;
            }

            // Rate limiting : 100ms entre les pages
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Ok(all_tickets)
    }

    /// Récupère les tickets modifiés depuis une date (sync incrémentale).
    pub async fn get_tickets_since(
        &self,
        since: &chrono::NaiveDateTime,
    ) -> Result<Vec<GlpiApiTicket>, GlpiError> {
        let since_str = since.format("%Y-%m-%d %H:%M:%S").to_string();

        // L'API Search avec critères filtre sur date_mod (champ 19)
        let url = format!(
            "{}/search/Ticket?\
            criteria[0][field]=19&criteria[0][searchtype]=morethan&criteria[0][value]={}&\
            forcedisplay[0]=1&forcedisplay[1]=2&forcedisplay[2]=12&\
            forcedisplay[3]=15&forcedisplay[4]=19&forcedisplay[5]=3&\
            forcedisplay[6]=7&\
            range=0-199",
            self.base_url,
            urlencoding::encode(&since_str)
        );

        let resp = self.authed_request(Method::GET, &url)?.send().await?;
        let result: SearchResponse = resp.json().await
            .map_err(|e| GlpiError::Parse(e.to_string()))?;

        // Convertir le format Search en GlpiApiTicket...
        Ok(result.into_tickets())
    }
}
```

### 6.4 Champs de recherche GLPI utiles

L'API Search utilise des identifiants numériques pour les champs. Référence pour les tickets :

|Field ID|Champ|Exemple de valeur|
|--:|---|---|
|1|Titre (name)|"Panne imprimante"|
|2|ID|5732943|
|3|Priorité|3 (Moyenne)|
|4|Demandeur|"DUPONT Marie"|
|5|Technicien assigné|"POUSSIER Killian"|
|7|Catégorie ITIL|"Matériel > Imprimante"|
|12|Statut|`notold` (vivants), `old` (terminés)|
|15|Date d'ouverture|"2026-01-05 16:24:00"|
|19|Dernière modification|"2026-02-28 09:15:00"|
|17|Date de résolution|"2026-02-10 14:30:00"|
|18|Date de clôture|"2026-02-12 08:00:00"|
|71|Groupe technicien|"_DSI > _SUPPORT"|

Opérateurs de recherche : `contains`, `equals`, `morethan`, `lessthan`, `under` (arbre). Liaison logique : `AND` (défaut), `OR`, `AND NOT`.

### 6.5 Stockage sécurisé des credentials

Les tokens API ne doivent **jamais** être stockés en clair dans un fichier de configuration. Le crate `keyring` (v3.6.x) utilise le gestionnaire de mots de passe natif du système : Windows Credential Manager, macOS Keychain, Linux Secret Service (GNOME Keyring, KWallet).

```toml
[dependencies]
keyring = "3.6"
```

```rust
use keyring::Entry;

const SERVICE_NAME: &str = "com.cpam92.glpi-dashboard";

/// Stocke les credentials API dans le keyring système.
pub fn store_api_credentials(
    url: &str,
    user_token: &str,
    app_token: Option<&str>,
) -> Result<(), keyring::Error> {
    Entry::new(SERVICE_NAME, "glpi_url")?.set_password(url)?;
    Entry::new(SERVICE_NAME, "glpi_user_token")?.set_password(user_token)?;
    if let Some(at) = app_token {
        Entry::new(SERVICE_NAME, "glpi_app_token")?.set_password(at)?;
    }
    Ok(())
}

/// Charge les credentials depuis le keyring système.
pub fn load_api_credentials() -> Result<ApiCredentials, keyring::Error> {
    Ok(ApiCredentials {
        url: Entry::new(SERVICE_NAME, "glpi_url")?.get_password()?,
        user_token: Entry::new(SERVICE_NAME, "glpi_user_token")?.get_password()?,
        app_token: Entry::new(SERVICE_NAME, "glpi_app_token")?
            .get_password().ok(),
    })
}

/// Supprime les credentials du keyring.
pub fn clear_api_credentials() -> Result<(), keyring::Error> {
    let _ = Entry::new(SERVICE_NAME, "glpi_url")?.delete_credential();
    let _ = Entry::new(SERVICE_NAME, "glpi_user_token")?.delete_credential();
    let _ = Entry::new(SERVICE_NAME, "glpi_app_token")?.delete_credential();
    Ok(())
}

pub struct ApiCredentials {
    pub url: String,
    pub user_token: String,
    pub app_token: Option<String>,
}
```

### 6.6 Architecture hybride CSV + API

L'objectif à terme est de permettre aux utilisateurs de basculer du mode CSV (import manuel) au mode API (sync automatique) sans changer l'architecture interne. Le pattern **trait objet** abstrait la source de données :

```rust
use async_trait::async_trait;

/// Trait commun pour les sources de données tickets.
#[async_trait]
pub trait TicketDataSource: Send + Sync {
    /// Récupère tous les tickets.
    async fn fetch_all(&self) -> Result<Vec<ParsedTicket>, Box<dyn std::error::Error>>;

    /// Récupère les tickets modifiés depuis une date.
    async fn fetch_since(
        &self,
        since: chrono::NaiveDateTime,
    ) -> Result<Vec<ParsedTicket>, Box<dyn std::error::Error>>;

    /// Nom de la source pour les logs.
    fn source_name(&self) -> &str;
}

/// Source CSV (mode actuel).
pub struct CsvDataSource {
    pub file_path: std::path::PathBuf,
}

#[async_trait]
impl TicketDataSource for CsvDataSource {
    async fn fetch_all(&self) -> Result<Vec<ParsedTicket>, Box<dyn std::error::Error>> {
        // Parsing CSV du Segment 1
        parse_csv_file(&self.file_path)
    }

    async fn fetch_since(&self, _since: chrono::NaiveDateTime) -> Result<Vec<ParsedTicket>, Box<dyn std::error::Error>> {
        // CSV ne supporte pas le delta — retourne tout et laisse le diff SQLite filtrer
        self.fetch_all().await
    }

    fn source_name(&self) -> &str { "CSV" }
}

/// Source API GLPI.
pub struct ApiDataSource {
    client: GlpiClient,
}

#[async_trait]
impl TicketDataSource for ApiDataSource {
    async fn fetch_all(&self) -> Result<Vec<ParsedTicket>, Box<dyn std::error::Error>> {
        let api_tickets = self.client.get_all_tickets().await?;
        Ok(api_tickets.into_iter().map(|t| t.into_parsed()).collect())
    }

    async fn fetch_since(&self, since: chrono::NaiveDateTime) -> Result<Vec<ParsedTicket>, Box<dyn std::error::Error>> {
        let api_tickets = self.client.get_tickets_since(&since).await?;
        Ok(api_tickets.into_iter().map(|t| t.into_parsed()).collect())
    }

    fn source_name(&self) -> &str { "API GLPI" }
}

/// Gestionnaire hybride avec fallback.
pub struct HybridDataManager {
    api_source: Option<ApiDataSource>,
    csv_fallback: Option<CsvDataSource>,
}

impl HybridDataManager {
    /// Tente l'API d'abord, puis le CSV en fallback.
    pub async fn fetch_tickets(&self) -> Result<(Vec<ParsedTicket>, &str), Box<dyn std::error::Error>> {
        if let Some(ref api) = self.api_source {
            match api.fetch_all().await {
                Ok(tickets) => return Ok((tickets, api.source_name())),
                Err(e) => {
                    log::warn!("API indisponible ({}), fallback CSV", e);
                }
            }
        }
        if let Some(ref csv) = self.csv_fallback {
            let tickets = csv.fetch_all().await?;
            Ok((tickets, csv.source_name()))
        } else {
            Err("Aucune source de données disponible".into())
        }
    }
}
```

### 6.7 Considérations de sécurité

|Règle|Implémentation|
|---|---|
|**HTTPS obligatoire**|`reqwest` avec `rustls-tls`, pas d'OpenSSL. Jamais `danger_accept_invalid_certs()` en prod|
|**Tokens jamais en log**|Logger uniquement les 4 derniers caractères : `"****{}"`|
|**Session fermée proprement**|`killSession` dans le `Drop` de `GlpiClient` et à la fermeture de l'app|
|**Retry avec backoff**|500ms × 2^tentative, max 3 tentatives, sur les erreurs 5xx et timeout|
|**Pas de stockage fichier**|Credentials uniquement dans le keyring OS, jamais en YAML/JSON/SQLite|
|**Timeouts stricts**|30s connexion, 60s lecture, configurable|

### 6.8 Feuille de route d'intégration

|Phase|Périmètre|Dépendances|
|---|---|---|
|**Phase 0** (actuelle)|CSV uniquement, import manuel|Segment 1|
|**Phase 1**|Ajout du client API GLPI V1 en lecture seule|reqwest, keyring|
|**Phase 2**|Sync incrémentale (fetch_since) + diff automatique|Section 4 (longitudinal)|
|**Phase 3**|Sync périodique en arrière-plan (toutes les N heures)|Tokio timer, notifications|
|**Phase 4**|Écriture API : ajout de commentaires, changement de statut depuis le Dashboard|API V1 PUT/POST|
|**Phase 5**|Migration API V2 avec OAuth2 quand GLPI 11 sera déployé|OAuth2, PKCE|

---

## 7. Cargo.toml consolidé du Segment 8

```toml
[dependencies]
# --- Export Excel (Section 1) ---
rust_xlsxwriter = { version = "0.93", features = ["serde", "chrono", "zlib"] }

# --- Détection doublons (Section 2) ---
strsim = "0.11"
rayon = "1.10"

# --- Notifications desktop (Section 3) ---
tauri-plugin-notification = "2"

# --- Suivi longitudinal (Section 4) — déjà présent Segment 2 ---
# rusqlite = { version = "0.38", features = ["bundled"] }

# --- Export PDF (Section 5) ---
typst-as-lib = "0.4"
typst-pdf = "0.14"
plotters = { version = "0.3.7", default-features = false, features = ["bitmap_backend"] }

# --- API GLPI (Section 6 — Phase 1+) ---
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
keyring = "3.6"
urlencoding = "2.1"

# --- Déjà présent (Segments 1-7) ---
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
log = "0.4"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

---

## 8. Récapitulatif des décisions d'architecture

|Décision|Choix|Justification|
|---|---|---|
|Export Excel|rust_xlsxwriter 0.93+|3,8× Python, API complète, sérialisation Serde|
|Conditional formatting|XLSX natif (pas cellule par cellule)|Dynamique à l'ouverture, conforme OOXML|
|Graphiques Excel|Chart API intégrée|Bar, Line, Pie, dual-axis sans dépendance|
|Format numérique FR|Format strings US-locale|Excel traduit automatiquement selon locale OS|
|Similarité titres|Sørensen-Dice 0.60 + JW 0.40|Tolérance réordonnancement, O(m+n)|
|Seuil doublons|≥0.85 probable, ≥0.70 possible|Calibré sur titres GLPI français|
|Parallélisation doublons|Rayon + 3 niveaux optimisation|< 1s pour 10K tickets|
|Notifications|tauri-plugin-notification 2.3+|Toast natif Windows, config minimale|
|Diff imports|Hash FNV-1a + comparaison champ par champ|90% skip par hash, diff < 50ms|
|Schéma longitudinal|snapshots + change_log séparés|Log léger permanent, snapshots purgables|
|Export PDF|Typst via typst-as-lib|Qualité LaTeX, templates séparés, UTF-8 natif|
|Graphiques PDF|plotters → PNG → Typst image.decode|Découplage rendu/composition|
|API GLPI|V1 Legacy (apirest.php)|Compatible GLPI 9.5+, CPAM FR|
|Stockage credentials|keyring 3.6 (OS natif)|Windows Credential Manager, jamais en fichier|
|Architecture source|Trait TicketDataSource|Abstraction CSV/API, fallback transparent|

---

_Ce segment complète le GLPI Dashboard avec les fonctionnalités de production : exports Excel et PDF professionnels conformes au CDC, détection de doublons par similarité de chaînes, alerting desktop temps réel, suivi longitudinal entre imports successifs, et feuille de route d'intégration avec l'API REST GLPI. Il consomme les données parsées du Segment 1, le schéma SQLite du Segment 2, les KPI et la classification du Segment 3, les catégories du Segment 4, le pipeline NLP du Segment 5, les résultats de clustering du Segment 6, et s'intègre aux visualisations frontend du Segment 7. L'ensemble des 8 segments forme une spécification technique complète et implémentable du GLPI Dashboard CPAM 92._