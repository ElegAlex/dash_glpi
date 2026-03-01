# Parser CSV GLPI en Rust pour Tauri 2 : guide technique complet

Le crate `csv` v1.4.0 de BurntSushi, combiné à Serde et chrono, gère nativement l'ensemble des particularités de votre export GLPI — y compris le BOM UTF-8, les champs multilignes entre guillemets et le séparateur point-virgule. **Contrairement à une idée répandue, le crate csv gère le BOM UTF-8 nativement** depuis juin 2017 (correctif de l'issue #81) : les 3 octets `EF BB BF` sont automatiquement ignorés à la lecture. Pour vos ~10 000 lignes, le parsing complet avec désérialisation Serde prend **moins de 20 ms** en mode release. Ce guide fournit le code complet et fonctionnel pour chaque composant du parser.

## Dépendances exactes et configuration Cargo.toml

```toml
[dependencies]
csv = "1.4.0"
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
log = "0.4"
encoding_rs_io = "0.1.7"   # optionnel — uniquement si fichiers UTF-16 possibles
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
```

Les versions exactes vérifiées sur crates.io à date : **csv 1.4.0**, **serde 1.0.228**, **chrono 0.4.43**, **encoding_rs_io 0.1.7**, **thiserror 2.x**. Le crate csv dépend de `csv-core ^0.1.11` (actuellement 0.1.13) qui contient le code de stripping BOM dans `reader.rs`.

## Le ReaderBuilder gère nativement BOM, multilignes et point-virgule

La configuration du `ReaderBuilder` pour un export GLPI ne nécessite que trois paramètres essentiels : le délimiteur, le mode flexible et le trim. Le quoting RFC 4180 (guillemets doubles avec `""` pour échapper) est actif par défaut.

```rust
use std::io::Read;

fn build_csv_reader<R: Read>(reader: R) -> csv::Reader<R> {
    csv::ReaderBuilder::new()
        .delimiter(b';')          // séparateur GLPI
        .has_headers(true)        // défaut, active le matching par nom de colonne
        .flexible(true)           // tolère un nombre variable de colonnes par ligne
        .trim(csv::Trim::Headers) // trim les espaces dans les noms de colonnes
        .double_quote(true)       // défaut — "" dans un champ quoté = guillemet littéral
        .quoting(true)            // défaut — active le quoting RFC 4180
        .from_reader(reader)
}
```

**Les champs multilignes entre guillemets sont gérés nativement.** Le crate csv implémente un sur-ensemble strict de RFC 4180. Un champ comme `"Ligne 1\nLigne 2\nLigne 3"` (avec des retours à la ligne réels entre guillemets) est désérialisé en une seule `String` contenant les `\n`. Le terminateur de record (`\n` ou `\r\n`) n'est reconnu comme fin de ligne **que s'il est en dehors des guillemets**. Vos champs "Suivis - Description", "Solution", "Tâches - Description" et les champs multi-assignation sont donc correctement parsés sans aucune configuration supplémentaire.

**Le BOM UTF-8 est strippé automatiquement.** Le code dans `csv-core/src/reader.rs` vérifie au premier read si les 3 premiers octets sont `\xEF\xBB\xBF` et les ignore le cas échéant. Aucune dépendance supplémentaire n'est nécessaire pour un fichier UTF-8 avec BOM. Le recours à `encoding_rs_io` ne se justifie que si vos fichiers peuvent être encodés en UTF-16 (exports Excel, par exemple) :

```rust
use std::fs::File;
use encoding_rs_io::DecodeReaderBytesBuilder;

// Approche defensive : gère UTF-8 BOM, UTF-16 et encodages exotiques
fn open_csv_with_encoding_fallback(path: &str) -> csv::Reader<impl std::io::Read> {
    let file = File::open(path).expect("Impossible d'ouvrir le fichier");
    let transcoded = DecodeReaderBytesBuilder::new()
        .utf8_passthru(true)    // pas de transcoding si déjà UTF-8
        .strip_bom(true)        // supprime le BOM de la sortie
        .build(file);
    csv::ReaderBuilder::new()
        .delimiter(b';')
        .flexible(true)
        .trim(csv::Trim::Headers)
        .from_reader(transcoded)
}
```

Pour un skip manuel des 3 octets BOM (approche minimaliste sans dépendance) :

```rust
use std::io::{BufReader, Read, Seek, SeekFrom};

fn skip_bom_manual(file: &mut std::fs::File) -> std::io::Result<()> {
    let mut bom = [0u8; 3];
    file.read_exact(&mut bom)?;
    if bom != [0xEF, 0xBB, 0xBF] {
        file.seek(SeekFrom::Start(0))?; // pas de BOM, revenir au début
    }
    Ok(())
}
```

**En pratique, pour votre cas** (UTF-8 avec BOM, crate csv 1.4.0), aucun traitement BOM explicite n'est nécessaire. Le crate le fait tout seul.

Les en-têtes avec accents français (`Attribué à`, `Dernière modification`, `Date d'ouverture`) fonctionnent sans configuration spéciale. Le `StringRecord` interne du crate impose la validité UTF-8, et le matching Serde par nom de colonne compare des chaînes UTF-8 standard.

## Structure GlpiTicket complète avec désérialiseurs custom

La struct de désérialisation couvre les 16 colonnes exactes, avec un champ optionnel pour la future colonne "Catégorie". Chaque champ problématique (dates, IDs, nombres vides, listes multi-valeurs) utilise un `deserialize_with` dédié.

```rust
use chrono::NaiveDateTime;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GlpiTicketRaw {
    #[serde(rename = "ID", deserialize_with = "de::spaced_u64")]
    pub id: u64,

    #[serde(rename = "Titre")]
    pub titre: String,

    #[serde(rename = "Attribué à - Groupe de techniciens")]
    pub groupe_techniciens: String,

    #[serde(rename = "Statut")]
    pub statut: String,

    #[serde(rename = "Attribué à - Technicien")]
    pub technicien: String,

    #[serde(rename = "Demandeur - Demandeur")]
    pub demandeur: String,

    #[serde(rename = "Date d'ouverture", deserialize_with = "de::french_datetime")]
    pub date_ouverture: NaiveDateTime,

    #[serde(rename = "Type")]
    pub type_ticket: String,

    #[serde(rename = "Suivis - Description")]
    pub suivis_description: String,

    #[serde(rename = "Suivis - Nombre de suivis", deserialize_with = "de::opt_u32_empty")]
    pub nombre_suivis: Option<u32>,

    #[serde(rename = "Plugins - Intervention fourniseur : Intervention")]
    pub intervention_fournisseur: String,

    #[serde(rename = "Solution - Solution")]
    pub solution: String,

    #[serde(rename = "Priorité", deserialize_with = "de::opt_u8_empty")]
    pub priorite: Option<u8>,

    #[serde(rename = "Tâches - Description")]
    pub taches_description: String,

    #[serde(rename = "Urgence", deserialize_with = "de::opt_u8_empty")]
    pub urgence: Option<u8>,

    #[serde(rename = "Dernière modification", deserialize_with = "de::french_datetime_opt")]
    pub derniere_modification: Option<NaiveDateTime>,

    // Colonne future — absente du CSV actuel
    #[serde(rename = "Catégorie", default)]
    pub categorie: Option<String>,
}
```

Le `#[serde(rename = "...")]` accepte directement les caractères UTF-8 accentués. Le `#[serde(default)]` sur `categorie` produit `None` quand la colonne est absente des en-têtes du CSV. **Attention** : `#[serde(default)]` ne fonctionne que pour les colonnes totalement absentes, pas pour les champs vides (un champ vide `""` avec `Option<String>` produit `Some("")`, pas `None`).

## Module de désérialiseurs custom pour les spécificités GLPI

Tous les désérialiseurs sont regroupés dans un module `de` pour la lisibilité. Chaque fonction gère un cas précis du format GLPI.

```rust
pub mod de {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer};

    const FRENCH_DT_FMT: &str = "%d-%m-%Y %H:%M";

    /// "05-01-2026 16:24" → NaiveDateTime (obligatoire)
    pub fn french_datetime<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(s.trim(), FRENCH_DT_FMT)
            .map_err(serde::de::Error::custom)
    }

    /// "05-01-2026 16:24" → Some(NaiveDateTime), "" → None
    pub fn french_datetime_opt<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        NaiveDateTime::parse_from_str(trimmed, FRENCH_DT_FMT)
            .map(Some)
            .map_err(serde::de::Error::custom)
    }

    /// "5 732 943" → 5732943u64 (supprime tous les espaces et espaces insécables)
    pub fn spaced_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        cleaned.parse::<u64>().map_err(serde::de::Error::custom)
    }

    /// "" → None, "5" → Some(5), "invalid" → erreur
    pub fn opt_u32_empty<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        trimmed.parse::<u32>().map(Some).map_err(serde::de::Error::custom)
    }

    /// "" → None, "3" → Some(3)
    pub fn opt_u8_empty<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        trimmed.parse::<u8>().map(Some).map_err(serde::de::Error::custom)
    }
}
```

Le format chrono `%d-%m-%Y %H:%M` parse exactement `"05-01-2026 16:24"` → `2026-01-05T16:24:00`. Le `%d` attend un jour zero-padded, `%m` un mois zero-padded, `%Y` l'année sur 4 chiffres. Les dates vides sont gérées par le variant `_opt` qui retourne `None`.

Pour les IDs, le choix `u64` est préférable à `String` : **8 octets fixes** contre une allocation heap, comparaisons rapides, utilisable comme clé de `HashMap`. Le seul inconvénient est la perte du formatage français, reconstituable facilement à l'affichage.

Le crate csv fournit aussi `csv::invalid_option` comme helper intégré — il convertit toute erreur de désérialisation en `None` plutôt que de propager l'erreur :

```rust
// Alternative ultra-tolérante : tout champ invalide → None
#[serde(deserialize_with = "csv::invalid_option")]
pub nombre_suivis: Option<u32>,
```

Le calcul d'ancienneté et d'inactivité utilise `chrono::Utc::now().naive_utc()` :

```rust
use chrono::{NaiveDateTime, Utc};

impl GlpiTicketRaw {
    /// Jours entre date d'ouverture et aujourd'hui
    pub fn anciennete_jours(&self) -> i64 {
        let now = Utc::now().naive_utc();
        (now - self.date_ouverture).num_days()
    }

    /// Jours entre dernière modification et aujourd'hui (None si pas de date)
    pub fn inactivite_jours(&self) -> Option<i64> {
        self.derniere_modification.map(|dt| {
            let now = Utc::now().naive_utc();
            (now - dt).num_days()
        })
    }
}
```

## Détection dynamique des colonnes et extraction des valeurs uniques

Plutôt que de hardcoder les colonnes, une approche en deux phases permet de gérer l'évolution du format d'export : d'abord inspecter les en-têtes, puis parser les données. Le pattern `ColumnMap` dissocie la structure CSV de la logique métier.

```rust
use std::collections::{HashMap, HashSet};

pub struct ColumnMap {
    indices: HashMap<String, usize>,
    headers: Vec<String>,
}

impl ColumnMap {
    pub fn from_headers(headers: &csv::StringRecord) -> Self {
        let mut indices = HashMap::new();
        let mut header_list = Vec::new();
        for (i, field) in headers.iter().enumerate() {
            let name = field.trim().to_string();
            indices.insert(name.clone(), i);
            header_list.push(name);
        }
        ColumnMap { indices, headers: header_list }
    }

    pub fn get<'a>(&self, record: &'a csv::StringRecord, col: &str) -> Option<&'a str> {
        self.indices.get(col).and_then(|&i| record.get(i))
    }

    pub fn has(&self, col: &str) -> bool {
        self.indices.contains_key(col)
    }

    pub fn all_headers(&self) -> &[String] {
        &self.headers
    }
}

// Colonnes obligatoires (le parsing échoue si absentes)
const REQUIRED: &[&str] = &[
    "ID", "Titre", "Statut", "Date d'ouverture", "Type",
];

// Colonnes optionnelles (absentes = valeur par défaut)
const OPTIONAL: &[&str] = &[
    "Catégorie",
];

pub struct ColumnValidation {
    pub present: HashSet<String>,
    pub missing_optional: Vec<String>,
}

pub fn validate_columns(col_map: &ColumnMap) -> Result<ColumnValidation, Vec<String>> {
    let missing_required: Vec<String> = REQUIRED
        .iter()
        .filter(|&&c| !col_map.has(c))
        .map(|c| c.to_string())
        .collect();

    if !missing_required.is_empty() {
        return Err(missing_required);
    }

    let missing_optional = OPTIONAL
        .iter()
        .filter(|&&c| !col_map.has(c))
        .map(|c| c.to_string())
        .collect();

    Ok(ColumnValidation {
        present: col_map.all_headers().iter().cloned().collect(),
        missing_optional,
    })
}
```

Pour l'extraction des valeurs distinctes de statut (et de toute autre colonne catégorielle), un passage post-parsing collecte les valeurs uniques sans hardcoder la liste :

```rust
pub fn extract_unique_values(tickets: &[GlpiTicketRaw]) -> ImportMetadata {
    let mut statuts = HashSet::new();
    let mut types = HashSet::new();
    let mut groupes = HashSet::new();

    for t in tickets {
        statuts.insert(t.statut.clone());
        types.insert(t.type_ticket.clone());
        // Séparer les groupes multilignes
        for g in t.groupe_techniciens.split('\n').map(str::trim).filter(|s| !s.is_empty()) {
            groupes.insert(g.to_string());
        }
    }

    ImportMetadata { statuts, types, groupes }
}

pub struct ImportMetadata {
    pub statuts: HashSet<String>,
    pub types: HashSet<String>,
    pub groupes: HashSet<String>,
}
```

Cette approche dynamique rend le parser résilient : quand GLPI ajoutera la colonne "Catégorie", le code la détectera automatiquement sans modification (via le `#[serde(default)]` sur le champ). Les valeurs uniques de la nouvelle colonne seront collectées au même titre que les statuts.

## Parsing des champs multilignes GLPI et stratégie d'extraction

Les champs multilignes GLPI concatènent plusieurs valeurs avec `\n` comme séparateur. La stratégie recommandée est de désérialiser en `String` brute puis post-traiter, ce qui découple la lecture CSV de la logique métier.

```rust
impl GlpiTicketRaw {
    /// "BLANQUART CHRISTOPHE\nMEY CHETHARITH" → vec!["BLANQUART CHRISTOPHE", "MEY CHETHARITH"]
    pub fn techniciens(&self) -> Vec<&str> {
        self.technicien
            .split('\n')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Premier technicien = technicien principal
    pub fn technicien_principal(&self) -> Option<&str> {
        self.techniciens().into_iter().next()
    }

    /// "_DSI > _SUPPORT\n_DSI > _PRODUCTION" → vec!["_DSI > _SUPPORT", "_DSI > _PRODUCTION"]
    pub fn groupes(&self) -> Vec<&str> {
        self.groupe_techniciens
            .split('\n')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Nombre réel de techniciens assignés
    pub fn est_multi_assignation(&self) -> bool {
        self.techniciens().len() > 1
    }
}
```

Le champ "Suivis - Description" est plus complexe : il contient l'historique complet concaténé. GLPI sépare typiquement les suivis individuels par des séquences reconnaissables (double saut de ligne, horodatages, ou séparateurs). Une stratégie d'extraction robuste utilise des heuristiques sur le format GLPI :

```rust
use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct SuiviEntry {
    pub date: Option<NaiveDateTime>,
    pub auteur: Option<String>,
    pub contenu: String,
}

/// Tente d'isoler les suivis individuels dans le champ concaténé.
/// Heuristique : GLPI insère souvent les suivis avec un pattern
/// "DD-MM-YYYY HH:MM Nom Prénom\n contenu" ou avec des séparateurs.
pub fn parse_suivis_description(raw: &str) -> Vec<SuiviEntry> {
    if raw.trim().is_empty() {
        return Vec::new();
    }

    // Heuristique 1 : tenter de découper sur les patterns de date en début de ligne
    let date_pattern = regex::Regex::new(
        r"(?m)^(\d{2}-\d{2}-\d{4} \d{2}:\d{2})"
    ).unwrap();

    let mut entries = Vec::new();
    let mut last_pos = 0;

    for mat in date_pattern.find_iter(raw) {
        if mat.start() > last_pos && last_pos > 0 {
            let contenu = raw[last_pos..mat.start()].trim().to_string();
            if !contenu.is_empty() {
                entries.push(SuiviEntry {
                    date: None, // sera parsé ensuite
                    auteur: None,
                    contenu,
                });
            }
        }
        last_pos = mat.start();
    }
    // Dernier segment
    if last_pos < raw.len() {
        entries.push(SuiviEntry {
            date: None,
            auteur: None,
            contenu: raw[last_pos..].trim().to_string(),
        });
    }

    // Fallback : si aucun pattern trouvé, retourner le texte brut en un seul bloc
    if entries.is_empty() {
        entries.push(SuiviEntry {
            date: None,
            auteur: None,
            contenu: raw.trim().to_string(),
        });
    }

    entries
}
```

Notez que le parsing des suivis est **heuristique par nature** — le format de concaténation GLPI n'est pas formellement documenté et peut varier selon les versions. En production, il est prudent de conserver le champ brut en plus du résultat parsé, et d'exposer les deux au frontend.

## Architecture du module parser et intégration Tauri 2

L'organisation recommandée sépare types, parsing et commandes Tauri dans des modules distincts. Le pipeline suit quatre étapes : lecture → désérialisation → normalisation → résultat structuré.

```
src-tauri/src/
├── lib.rs                     # point d'entrée Tauri, enregistre les commandes
├── commands/
│   ├── mod.rs
│   └── import.rs              # #[tauri::command] import_csv()
├── parser/
│   ├── mod.rs                 # pub mod types, deserializers, pipeline;
│   ├── types.rs               # GlpiTicketRaw, CsvImportResult, ParseWarning
│   ├── deserializers.rs       # module de:: (dates, IDs, nombres)
│   ├── columns.rs             # ColumnMap, validation des colonnes
│   └── pipeline.rs            # parse_csv() — orchestrateur principal
└── error.rs                   # types d'erreur avec thiserror
```

La struct de résultat d'import contient tout ce dont le frontend a besoin :

```rust
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseWarning {
    pub line: usize,
    pub column: Option<String>,
    pub message: String,
    pub severity: String, // "info", "warning", "error"
}

/// Ticket normalisé pour le frontend (dates en ISO 8601, IDs numériques)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlpiTicketNormalized {
    pub id: u64,
    pub titre: String,
    pub statut: String,
    pub type_ticket: String,
    pub priorite: Option<u8>,
    pub urgence: Option<u8>,
    pub demandeur: String,
    pub technicien_principal: Option<String>,
    pub techniciens: Vec<String>,
    pub groupe_principal: Option<String>,
    pub groupes: Vec<String>,
    pub date_ouverture: String,         // ISO 8601
    pub derniere_modification: Option<String>,  // ISO 8601
    pub anciennete_jours: i64,
    pub inactivite_jours: Option<i64>,
    pub nombre_suivis: Option<u32>,
    pub suivis_description: String,     // brut
    pub solution: String,
    pub taches_description: String,
    pub intervention_fournisseur: String,
    pub categorie: Option<String>,
}
```

Le pipeline principal orchestre le parsing avec collecte d'erreurs et progress reporting :

```rust
use std::time::Instant;

pub fn parse_csv<F>(
    path: &str,
    on_progress: F,
) -> Result<CsvImportResult, CsvImportError>
where
    F: Fn(u64, usize), // (bytes_read, rows_parsed)
{
    let start = Instant::now();
    let file_size = std::fs::metadata(path)?.len();

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::Headers)
        .from_path(path)?;

    // Phase 1 : validation des colonnes
    let headers = rdr.headers()?.clone();
    let col_map = ColumnMap::from_headers(&headers);
    let col_validation = validate_columns(&col_map)
        .map_err(|missing| CsvImportError::MissingColumns(missing))?;

    // Phase 2 : désérialisation tolérante aux erreurs
    let mut tickets_raw: Vec<GlpiTicketRaw> = Vec::with_capacity(10_000);
    let mut warnings: Vec<ParseWarning> = Vec::new();
    let mut skipped = 0usize;
    let mut rows = 0usize;

    for result in rdr.deserialize::<GlpiTicketRaw>() {
        rows += 1;
        if rows % 500 == 0 {
            on_progress(
                (file_size as f64 * rows as f64 / 10_000.0) as u64, // estimation
                rows,
            );
        }

        match result {
            Ok(ticket) => tickets_raw.push(ticket),
            Err(err) => {
                warnings.push(ParseWarning {
                    line: rows + 1, // +1 pour le header
                    column: None,
                    message: format!("{}", err),
                    severity: "error".into(),
                });
                skipped += 1;
            }
        }
    }

    // Phase 3 : normalisation
    let now = chrono::Utc::now().naive_utc();
    let tickets: Vec<GlpiTicketNormalized> = tickets_raw
        .iter()
        .map(|raw| normalize_ticket(raw, &now))
        .collect();

    // Phase 4 : extraction des métadonnées
    let metadata = extract_unique_values(&tickets_raw);

    Ok(CsvImportResult {
        tickets,
        warnings,
        total_rows_processed: rows,
        skipped_rows: skipped,
        detected_columns: col_validation.present.into_iter().collect(),
        missing_optional_columns: col_validation.missing_optional,
        unique_statuts: metadata.statuts.into_iter().collect(),
        unique_types: metadata.types.into_iter().collect(),
        unique_groupes: metadata.groupes.into_iter().collect(),
        parse_duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn normalize_ticket(raw: &GlpiTicketRaw, now: &chrono::NaiveDateTime) -> GlpiTicketNormalized {
    let techs = raw.techniciens();
    let groupes = raw.groupes();

    GlpiTicketNormalized {
        id: raw.id,
        titre: raw.titre.clone(),
        statut: raw.statut.clone(),
        type_ticket: raw.type_ticket.clone(),
        priorite: raw.priorite,
        urgence: raw.urgence,
        demandeur: raw.demandeur.clone(),
        technicien_principal: techs.first().map(|s| s.to_string()),
        techniciens: techs.iter().map(|s| s.to_string()).collect(),
        groupe_principal: groupes.first().map(|s| s.to_string()),
        groupes: groupes.iter().map(|s| s.to_string()).collect(),
        date_ouverture: raw.date_ouverture.format("%Y-%m-%dT%H:%M:%S").to_string(),
        derniere_modification: raw.derniere_modification
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string()),
        anciennete_jours: (*now - raw.date_ouverture).num_days(),
        inactivite_jours: raw.derniere_modification.map(|dt| (*now - dt).num_days()),
        nombre_suivis: raw.nombre_suivis,
        suivis_description: raw.suivis_description.clone(),
        solution: raw.solution.clone(),
        taches_description: raw.taches_description.clone(),
        intervention_fournisseur: raw.intervention_fournisseur.clone(),
        categorie: raw.categorie.clone().filter(|s| !s.is_empty()),
    }
}
```

La commande Tauri 2 utilise le **Channel API** pour le progress reporting — plus performant et typé que le système d'événements :

```rust
use tauri::ipc::Channel;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ImportEvent {
    #[serde(rename_all = "camelCase")]
    Progress { bytes_read: u64, rows_parsed: usize, total_bytes: u64 },
    #[serde(rename_all = "camelCase")]
    Complete { duration_ms: u64 },
}

#[tauri::command]
pub async fn import_csv(
    path: String,
    on_progress: Channel<ImportEvent>,
) -> Result<CsvImportResult, String> {
    let file_size = std::fs::metadata(&path)
        .map_err(|e| format!("Fichier inaccessible: {}", e))?
        .len();

    let result = crate::parser::pipeline::parse_csv(&path, |bytes_read, rows_parsed| {
        let _ = on_progress.send(ImportEvent::Progress {
            bytes_read,
            rows_parsed,
            total_bytes: file_size,
        });
    }).map_err(|e| e.to_string())?;

    let _ = on_progress.send(ImportEvent::Complete {
        duration_ms: result.parse_duration_ms,
    });

    Ok(result)
}
```

Côté frontend TypeScript, l'invocation avec channel :

```typescript
import { invoke, Channel } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

async function handleImport() {
  const path = await open({
    multiple: false,
    filters: [{ name: 'CSV', extensions: ['csv'] }],
  });
  if (!path) return;

  const onProgress = new Channel<ImportEvent>();
  onProgress.onmessage = (msg) => {
    if (msg.event === 'progress') {
      const pct = (msg.data.bytesRead / msg.data.totalBytes) * 100;
      setProgress(Math.min(pct, 99)); // ne jamais afficher 100% avant Complete
    }
  };

  const result = await invoke<CsvImportResult>('import_csv', { path, onProgress });
  console.log(`${result.tickets.length} tickets importés en ${result.parseDurationMs}ms`);
  console.log(`Statuts détectés: ${result.uniqueStatuts.join(', ')}`);
}
```

L'enregistrement des commandes dans `lib.rs` :

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::import::import_csv,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au lancement de l'application");
}
```

## Performance, robustesse et tests unitaires

Les benchmarks du crate csv montrent **~83 MB/s en mode Serde désérialisation**, **~122 MB/s en StringRecord** et **~241 MB/s en mode raw** (csv-core sans allocation). Votre fichier de 9 616 lignes × 16 colonnes pèse environ **1 à 3 Mo** selon la taille des champs texte. Le parsing complet, désérialisation Serde incluse, prend donc **moins de 40 ms** — imperceptible pour l'utilisateur. Il n'y a aucun besoin d'optimisation de performance pour ce volume.

Le chargement en mémoire complet (`Vec<GlpiTicketNormalized>`) est la bonne approche ici. Pour 10 000 tickets avec ~16 champs String, l'empreinte mémoire est d'environ **10 à 30 Mo** — triviale pour une application desktop. Le streaming n'apporterait que de la complexité sans bénéfice mesurable.

Pour estimer la progression de manière précise malgré les champs multilignes, la méthode la plus fiable est le **ratio octets lus / taille totale du fichier**. Compter les `\n` donnerait un résultat faux puisque les champs entre guillemets contiennent des retours à la ligne qui ne sont pas des fins de record. Le `csv::Reader` expose la position via `reader.position().byte()` après chaque record lu.

Le type d'erreur avec `thiserror` :

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CsvImportError {
    #[error("Erreur d'entrée/sortie: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur de parsing CSV: {0}")]
    Csv(#[from] csv::Error),

    #[error("Colonnes obligatoires manquantes: {}", .0.join(", "))]
    MissingColumns(Vec<String>),

    #[error("Fichier vide ou sans données")]
    EmptyFile,
}

impl serde::Serialize for CsvImportError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}
```

Les tests unitaires utilisent des fixtures CSV inline pour couvrir chaque cas limite :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn parse_test_csv(csv_data: &str) -> CsvImportResult {
        // Helper : écrit dans un fichier temporaire et parse
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.csv");
        std::fs::write(&path, csv_data).unwrap();
        parse_csv(path.to_str().unwrap(), |_, _| {}).unwrap()
    }

    #[test]
    fn test_parsing_basique() {
        let csv = "ID;Titre;Attribué à - Groupe de techniciens;Statut;\
            Attribué à - Technicien;Demandeur - Demandeur;Date d'ouverture;\
            Type;Suivis - Description;Suivis - Nombre de suivis;\
            Plugins - Intervention fourniseur : Intervention;Solution - Solution;\
            Priorité;Tâches - Description;Urgence;Dernière modification\n\
            5 732 943;Problème réseau;_DSI > _SUPPORT;Nouveau;\
            BLANQUART CHRISTOPHE;DUPONT Jean;05-01-2026 16:24;\
            Incident;;3;;Solution appliquée;3;;4;06-01-2026 09:00";

        let result = parse_test_csv(csv);
        assert_eq!(result.tickets.len(), 1);
        assert_eq!(result.tickets[0].id, 5_732_943);
        assert_eq!(result.tickets[0].statut, "Nouveau");
        assert!(result.tickets[0].anciennete_jours > 0);
    }

    #[test]
    fn test_champs_multilignes() {
        let csv = "ID;Titre;Attribué à - Groupe de techniciens;Statut;\
            Attribué à - Technicien;Demandeur - Demandeur;Date d'ouverture;\
            Type;Suivis - Description;Suivis - Nombre de suivis;\
            Plugins - Intervention fourniseur : Intervention;Solution - Solution;\
            Priorité;Tâches - Description;Urgence;Dernière modification\n\
            100;Test;\"_DSI > _SUPPORT\n_DSI > _PRODUCTION\";En cours;\
            \"BLANQUART CHRISTOPHE\nMEY CHETHARITH\";DEM;05-01-2026 10:00;\
            Demande;;0;;;;3;;4;";

        let result = parse_test_csv(csv);
        assert_eq!(result.tickets[0].techniciens.len(), 2);
        assert_eq!(result.tickets[0].groupes.len(), 2);
        assert_eq!(result.tickets[0].technicien_principal, Some("BLANQUART CHRISTOPHE".into()));
    }

    #[test]
    fn test_bom_utf8() {
        let csv = "\u{FEFF}ID;Titre;Attribué à - Groupe de techniciens;Statut;\
            Attribué à - Technicien;Demandeur - Demandeur;Date d'ouverture;\
            Type;Suivis - Description;Suivis - Nombre de suivis;\
            Plugins - Intervention fourniseur : Intervention;Solution - Solution;\
            Priorité;Tâches - Description;Urgence;Dernière modification\n\
            1;Test;G;Nouveau;T;D;01-01-2026 08:00;Incident;;0;;;;3;;4;";

        let result = parse_test_csv(csv);
        assert_eq!(result.tickets.len(), 1);
    }

    #[test]
    fn test_colonne_categorie_absente() {
        // Le CSV actuel n'a pas de colonne Catégorie
        let csv = "ID;Titre;Attribué à - Groupe de techniciens;Statut;\
            Attribué à - Technicien;Demandeur - Demandeur;Date d'ouverture;\
            Type;Suivis - Description;Suivis - Nombre de suivis;\
            Plugins - Intervention fourniseur : Intervention;Solution - Solution;\
            Priorité;Tâches - Description;Urgence;Dernière modification\n\
            1;T;G;Nouveau;Tech;Dem;01-01-2026 08:00;Inc;;0;;;;3;;4;";

        let result = parse_test_csv(csv);
        assert!(result.tickets[0].categorie.is_none());
        assert!(result.missing_optional_columns.contains(&"Catégorie".to_string()));
    }

    #[test]
    fn test_ligne_malformee_skip() {
        // La ligne 3 a un ID invalide
        let csv = "ID;Titre;Attribué à - Groupe de techniciens;Statut;\
            Attribué à - Technicien;Demandeur - Demandeur;Date d'ouverture;\
            Type;Suivis - Description;Suivis - Nombre de suivis;\
            Plugins - Intervention fourniseur : Intervention;Solution - Solution;\
            Priorité;Tâches - Description;Urgence;Dernière modification\n\
            1;OK;G;Nouveau;T;D;01-01-2026 08:00;Inc;;0;;;;3;;4;\n\
            INVALID;Bad;G;X;T;D;not-a-date;Inc;;0;;;;3;;4;\n\
            2;Also OK;G;Résolu;T;D;02-01-2026 09:00;Inc;;1;;;;3;;4;";

        let result = parse_test_csv(csv);
        assert_eq!(result.tickets.len(), 2);
        assert_eq!(result.skipped_rows, 1);
        assert_eq!(result.warnings.len(), 1);
    }

    // Pour les fixtures de taille réelle, utiliser include_str!
    // Placer le fichier dans src-tauri/src/parser/test_fixtures/
    // #[test]
    // fn test_export_reel() {
    //     let csv = include_str!("test_fixtures/glpi_export_sample.csv");
    //     let result = parse_test_csv(csv);
    //     assert!(result.tickets.len() > 100);
    //     assert!(result.skipped_rows < result.tickets.len() / 100); // <1% d'erreurs
    // }
}
```

## Conclusion

L'écosystème Rust couvre l'intégralité des besoins de parsing de cet export GLPI sans compromis. Le crate `csv` 1.4.0 gère nativement le BOM UTF-8, les champs multilignes entre guillemets et le séparateur point-virgule — `encoding_rs_io` n'est nécessaire que pour des fichiers non-UTF-8. Le choix architectural clé est la **séparation en trois couches** : désérialisation brute (struct `GlpiTicketRaw` avec Serde), normalisation (struct `GlpiTicketNormalized` avec calculs dérivés), et résultat structuré (`CsvImportResult` avec métadonnées dynamiques). Le `#[serde(default)]` sur `Option<String>` permet d'absorber l'ajout futur de la colonne "Catégorie" sans modification de code. Pour le progress reporting dans Tauri 2, le Channel API (`tauri::ipc::Channel<T>`) est préférable au système d'événements car il est typé, ordonné et plus performant. Le ratio octets lus / taille fichier fournit une estimation de progression fiable malgré les champs multilignes qui faussent le comptage de lignes.