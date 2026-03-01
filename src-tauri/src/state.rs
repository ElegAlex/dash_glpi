use rusqlite::Connection;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Option<Connection>>,
}

pub trait DbAccess {
    fn db<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>;

    fn db_mut<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut Connection) -> Result<T, rusqlite::Error>;
}

impl DbAccess for AppState {
    fn db<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let guard = self.db.lock().map_err(|e| format!("Mutex poisoned: {}", e))?;
        let conn = guard.as_ref().ok_or("Base de données non initialisée")?;
        f(conn).map_err(|e| format!("Erreur SQLite: {}", e))
    }

    fn db_mut<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut Connection) -> Result<T, rusqlite::Error>,
    {
        let mut guard = self.db.lock().map_err(|e| format!("Mutex poisoned: {}", e))?;
        let conn = guard.as_mut().ok_or("Base de données non initialisée")?;
        f(conn).map_err(|e| format!("Erreur SQLite: {}", e))
    }
}
