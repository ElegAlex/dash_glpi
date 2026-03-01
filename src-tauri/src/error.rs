use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Erreur d'entrée/sortie: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur CSV: {0}")]
    Csv(#[from] csv::Error),

    #[error("Erreur SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Erreur de sérialisation: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Colonnes obligatoires manquantes: {}", .0.join(", "))]
    MissingColumns(Vec<String>),

    #[error("Fichier vide ou sans données")]
    EmptyFile,

    #[error("Import introuvable: {0}")]
    ImportNotFound(i64),

    #[error("{0}")]
    Custom(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
