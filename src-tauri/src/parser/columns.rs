use std::collections::HashMap;

use crate::error::AppError;

/// Colonnes obligatoires — l'import échoue si l'une d'elles est absente.
const REQUIRED: &[&str] = &["ID", "Titre", "Statut", "Date d'ouverture", "Type"];

/// Colonnes optionnelles — absentes = valeur par défaut, signalées dans le résultat.
const OPTIONAL: &[&str] = &["Catégorie"];

/// Maps column names to their index in a CSV record.
pub struct ColumnMap {
    indices: HashMap<String, usize>,
    headers: Vec<String>,
}

impl ColumnMap {
    /// Build a ColumnMap from the CSV header record.
    /// Header fields are trimmed of surrounding whitespace.
    pub fn from_headers(headers: &csv::StringRecord) -> Self {
        let mut indices = HashMap::new();
        let mut header_list = Vec::new();
        for (i, field) in headers.iter().enumerate() {
            let name = field.trim().to_string();
            indices.insert(name.clone(), i);
            header_list.push(name);
        }
        ColumnMap {
            indices,
            headers: header_list,
        }
    }

    /// Get the value of a named column from a record.
    pub fn get<'a>(&self, record: &'a csv::StringRecord, col: &str) -> Option<&'a str> {
        self.indices.get(col).and_then(|&i| record.get(i))
    }

    /// Returns true if the column is present in the CSV headers.
    pub fn has(&self, col: &str) -> bool {
        self.indices.contains_key(col)
    }

    /// All header names in order.
    pub fn all_headers(&self) -> &[String] {
        &self.headers
    }
}

/// Result of column validation.
#[derive(Debug)]
pub struct ColumnValidation {
    /// All column names present in the CSV.
    pub present: Vec<String>,
    /// Optional columns that are absent from the CSV.
    pub missing_optional: Vec<String>,
}

/// Validate that all required columns are present.
/// Returns `AppError::MissingColumns` if any required column is absent.
pub fn validate_columns(col_map: &ColumnMap) -> Result<ColumnValidation, AppError> {
    let missing_required: Vec<String> = REQUIRED
        .iter()
        .filter(|&&c| !col_map.has(c))
        .map(|c| c.to_string())
        .collect();

    if !missing_required.is_empty() {
        return Err(AppError::MissingColumns(missing_required));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_headers(cols: &[&str]) -> csv::StringRecord {
        csv::StringRecord::from(cols.to_vec())
    }

    #[test]
    fn test_column_map_basic() {
        let headers = make_headers(&["ID", "Titre", "Statut"]);
        let cm = ColumnMap::from_headers(&headers);
        assert!(cm.has("ID"));
        assert!(cm.has("Titre"));
        assert!(!cm.has("Missing"));
    }

    #[test]
    fn test_column_map_get() {
        let headers = make_headers(&["ID", "Titre"]);
        let cm = ColumnMap::from_headers(&headers);
        let record = csv::StringRecord::from(vec!["42", "Mon titre"]);
        assert_eq!(cm.get(&record, "ID"), Some("42"));
        assert_eq!(cm.get(&record, "Titre"), Some("Mon titre"));
        assert_eq!(cm.get(&record, "Missing"), None);
    }

    #[test]
    fn test_validate_columns_ok() {
        let headers = make_headers(&[
            "ID",
            "Titre",
            "Statut",
            "Date d'ouverture",
            "Type",
            "Catégorie",
        ]);
        let cm = ColumnMap::from_headers(&headers);
        let val = validate_columns(&cm).unwrap();
        assert!(val.missing_optional.is_empty());
    }

    #[test]
    fn test_validate_columns_missing_required() {
        let headers = make_headers(&["Titre", "Statut"]);
        let cm = ColumnMap::from_headers(&headers);
        let err = validate_columns(&cm).unwrap_err();
        match err {
            AppError::MissingColumns(cols) => {
                assert!(cols.contains(&"ID".to_string()));
                assert!(cols.contains(&"Date d'ouverture".to_string()));
                assert!(cols.contains(&"Type".to_string()));
            }
            _ => panic!("Expected MissingColumns error"),
        }
    }

    #[test]
    fn test_validate_columns_missing_optional() {
        let headers = make_headers(&["ID", "Titre", "Statut", "Date d'ouverture", "Type"]);
        let cm = ColumnMap::from_headers(&headers);
        let val = validate_columns(&cm).unwrap();
        assert!(val.missing_optional.contains(&"Catégorie".to_string()));
    }

    #[test]
    fn test_column_map_trim_whitespace() {
        let headers = make_headers(&[" ID ", " Titre "]);
        let cm = ColumnMap::from_headers(&headers);
        assert!(cm.has("ID"));
        assert!(cm.has("Titre"));
    }
}
