# Mapping Colonnes CSV → DB — GLPI Dashboard

Source : Segments 1 & 2

---

## Vue d'ensemble

```
Colonne CSV française → GlpiTicketRaw (Serde) → GlpiTicketNormalized → Colonne SQLite tickets
```

---

## Mapping complet

| Colonne CSV (rename Serde) | Champ GlpiTicketRaw | Désérialiseur | Champ GlpiTicketNormalized | Colonne tickets SQLite |
|---|---|---|---|---|
| `ID` | `id: u64` | `de::spaced_u64` | `id: u64` | `id INTEGER` |
| `Titre` | `titre: String` | (défaut) | `titre: String` | `titre TEXT` |
| `Statut` | `statut: String` | (défaut) | `statut: String` | `statut TEXT` |
| `Type` | `type_ticket: String` | (défaut) | `type_ticket: String` | `type_ticket TEXT` |
| `Priorité` | `priorite: Option<u8>` | `de::opt_u8_empty` | `priorite: Option<u8>` | `priorite INTEGER` |
| `Urgence` | `urgence: Option<u8>` | `de::opt_u8_empty` | `urgence: Option<u8>` | `urgence INTEGER` |
| `Demandeur - Demandeur` | `demandeur: String` | (défaut) | `demandeur: String` | `demandeur TEXT` |
| `Date d'ouverture` | `date_ouverture: NaiveDateTime` | `de::french_datetime` | `date_ouverture: String` (ISO 8601) | `date_ouverture TEXT` |
| `Dernière modification` | `derniere_modification: Option<NaiveDateTime>` | `de::french_datetime_opt` | `derniere_modification: Option<String>` | `derniere_modification TEXT` |
| `Suivis - Nombre de suivis` | `nombre_suivis: Option<u32>` | `de::opt_u32_empty` | `nombre_suivis: Option<u32>` | `nombre_suivis INTEGER` |
| `Suivis - Description` | `suivis_description: String` | (défaut) | `suivis_description: String` | `suivis_description TEXT` |
| `Solution - Solution` | `solution: String` | (défaut) | `solution: String` | `solution TEXT` |
| `Tâches - Description` | `taches_description: String` | (défaut) | `taches_description: String` | `taches_description TEXT` |
| `Plugins - Intervention fourniseur : Intervention` | `intervention_fournisseur: String` | (défaut) | `intervention_fournisseur: String` | `intervention_fournisseur TEXT` |
| `Attribué à - Technicien` | `technicien: String` (multiligne) | (défaut) | `techniciens: Vec<String>` | `techniciens TEXT` (JSON) |
| `Attribué à - Groupe de techniciens` | `groupe_techniciens: String` (multiligne) | (défaut) | `groupes: Vec<String>` | `groupes TEXT` (JSON) |
| `Catégorie` | `categorie: Option<String>` | `#[serde(default)]` | `categorie: Option<String>` | `categorie TEXT` |

---

## Champs calculés (GlpiTicketNormalized → SQLite)

| Champ GlpiTicketNormalized | Source | Colonne SQLite |
|---|---|---|
| `technicien_principal: Option<String>` | `techniciens.first()` | `technicien_principal TEXT` |
| `groupe_principal: Option<String>` | `groupes.first()` | `groupe_principal TEXT` |
| `anciennete_jours: i64` | `now - date_ouverture` | `anciennete_jours INTEGER` |
| `inactivite_jours: Option<i64>` | `now - derniere_modification` | `inactivite_jours INTEGER` |
| — | split(groupe_principal, " > ")[0] | `groupe_niveau1 TEXT` |
| — | split(groupe_principal, " > ")[1] | `groupe_niveau2 TEXT` |
| — | split(groupe_principal, " > ")[2] | `groupe_niveau3 TEXT` |
| — | statut in VIVANTS | `est_vivant INTEGER` (0/1) |
| — | derniere_modification si terminé | `date_cloture_approx TEXT` |

---

## Désérialiseurs custom (module `de`)

```rust
const FRENCH_DT_FMT: &str = "%d-%m-%Y %H:%M";

// "05-01-2026 16:24" → NaiveDateTime
fn french_datetime<'de, D>(d: D) -> Result<NaiveDateTime, D::Error>

// "05-01-2026 16:24" → Some(NaiveDateTime), "" → None
fn french_datetime_opt<'de, D>(d: D) -> Result<Option<NaiveDateTime>, D::Error>

// "5 732 943" → 5732943u64
fn spaced_u64<'de, D>(d: D) -> Result<u64, D::Error>

// "" → None, "5" → Some(5)
fn opt_u32_empty<'de, D>(d: D) -> Result<Option<u32>, D::Error>

// "" → None, "3" → Some(3)
fn opt_u8_empty<'de, D>(d: D) -> Result<Option<u8>, D::Error>
```

---

## Colonnes obligatoires vs optionnelles

```rust
const REQUIRED: &[&str] = &[
    "ID", "Titre", "Statut", "Date d'ouverture", "Type",
];

const OPTIONAL: &[&str] = &[
    "Catégorie",
];
```

**Note** : `#[serde(default)]` sur `categorie` → `None` si colonne absente (pas si valeur vide).
Une valeur vide `""` avec `Option<String>` produit `Some("")`, pas `None` → filtrer avec `.filter(|s| !s.is_empty())` lors de la normalisation.

---

## CsvImportResult (retour pipeline)

```rust
pub struct CsvImportResult {
    pub tickets: Vec<GlpiTicketNormalized>,
    pub warnings: Vec<ParseWarning>,
    pub total_rows_processed: usize,
    pub skipped_rows: usize,
    pub detected_columns: Vec<String>,
    pub missing_optional_columns: Vec<String>,
    pub unique_statuts: Vec<String>,
    pub unique_types: Vec<String>,
    pub unique_groupes: Vec<String>,
    pub parse_duration_ms: u64,
}
```

---

## Configuration ReaderBuilder

```rust
csv::ReaderBuilder::new()
    .delimiter(b';')          // séparateur GLPI
    .has_headers(true)
    .flexible(true)           // tolère nombre variable de colonnes
    .trim(csv::Trim::Headers) // trim espaces dans noms de colonnes
    .double_quote(true)       // "" = guillemet littéral (défaut)
    .quoting(true)            // RFC 4180 (défaut)
    .from_path(path)?
```
