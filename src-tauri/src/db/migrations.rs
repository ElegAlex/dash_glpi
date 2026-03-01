use rusqlite::Connection;

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("sql/001_initial.sql"),
}];

pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    let current_version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    for migration in MIGRATIONS {
        if migration.version > current_version {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(migration.sql)?;
            tx.pragma_update(None, "user_version", migration.version)?;
            tx.commit()?;
            log::info!("Migration {} appliquée", migration.version);
        }
    }

    Ok(())
}
