use std::path::Path;

use rusqlite::{params, Connection};

use crate::error::{Error, Result};
use crate::model::Workflow;

const SCHEMA_VERSION: i32 = 1;

const CREATE_TABLES: &str = "
CREATE TABLE IF NOT EXISTS workflows (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    definition TEXT NOT NULL,
    active     INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
";

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::init(Connection::open(path)?, true)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?, false)
    }

    fn init(conn: Connection, wal: bool) -> Result<Self> {
        if wal {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        conn.execute_batch(CREATE_TABLES)?;
        Ok(Self { conn })
    }

    pub fn insert_workflow(&self, wf: &Workflow) -> Result<()> {
        if self.get_workflow(&wf.id)?.is_some() {
            return Err(Error::Duplicate { id: wf.id.clone() });
        }
        let now = now_millis();
        self.conn.execute(
            "INSERT INTO workflows (id, name, definition, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                wf.id,
                wf.name,
                serde_json::to_string(wf)?,
                wf.active as i64,
                now,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_workflow(&self, id: &str) -> Result<Option<Workflow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT definition, active FROM workflows WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => {
                let mut wf: Workflow = serde_json::from_str(&row.get::<_, String>(0)?)?;
                wf.active = row.get::<_, i64>(1)? != 0;
                Ok(Some(wf))
            }
            None => Ok(None),
        }
    }

    pub fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT definition, active FROM workflows ORDER BY created_at")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (definition, active) = row?;
            let mut wf: Workflow = serde_json::from_str(&definition)?;
            wf.active = active;
            out.push(wf);
        }
        Ok(out)
    }

    pub fn update_workflow(&self, wf: &Workflow) -> Result<()> {
        let updated = self.conn.execute(
            "UPDATE workflows SET name = ?2, definition = ?3, active = ?4, updated_at = ?5
             WHERE id = ?1",
            params![
                wf.id,
                wf.name,
                serde_json::to_string(wf)?,
                wf.active as i64,
                now_millis()
            ],
        )?;
        if updated == 0 {
            return Err(Error::NotFound { id: wf.id.clone() });
        }
        Ok(())
    }

    pub fn delete_workflow(&self, id: &str) -> Result<bool> {
        let deleted = self.conn.execute("DELETE FROM workflows WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }

    pub fn set_active(&self, id: &str, active: bool) -> Result<()> {
        let updated = self.conn.execute(
            "UPDATE workflows SET active = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, active as i64, now_millis()],
        )?;
        if updated == 0 {
            return Err(Error::NotFound { id: id.to_string() });
        }
        Ok(())
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample(id: &str) -> Workflow {
        Workflow {
            id: id.into(),
            name: "Sample".into(),
            knots: vec![
                crate::model::Knot::new("a", "trigger.webhook", json!({})),
                crate::model::Knot::new("b", "notify.log", json!({ "message": "hi" })),
            ],
            connections: vec![crate::model::Connection {
                from: "a".into(),
                from_output: 0,
                to: "b".into(),
                to_input: 0,
            }],
            active: false,
        }
    }

    #[test]
    fn insert_and_get_round_trip() {
        let store = Store::open_in_memory().unwrap();
        let wf = sample("w1");
        store.insert_workflow(&wf).unwrap();

        let loaded = store.get_workflow("w1").unwrap().unwrap();
        assert_eq!(loaded, wf);
        assert_eq!(store.get_workflow("missing").unwrap(), None);
    }

    #[test]
    fn insert_duplicate_fails() {
        let store = Store::open_in_memory().unwrap();
        store.insert_workflow(&sample("w1")).unwrap();
        let err = store.insert_workflow(&sample("w1")).unwrap_err();
        assert!(matches!(err, Error::Duplicate { .. }));
    }

    #[test]
    fn update_and_set_active() {
        let store = Store::open_in_memory().unwrap();
        store.insert_workflow(&sample("w1")).unwrap();

        let mut wf = store.get_workflow("w1").unwrap().unwrap();
        wf.name = "Renamed".into();
        store.update_workflow(&wf).unwrap();
        assert_eq!(store.get_workflow("w1").unwrap().unwrap().name, "Renamed");

        store.set_active("w1", true).unwrap();
        assert!(store.get_workflow("w1").unwrap().unwrap().active);
    }

    #[test]
    fn update_missing_fails() {
        let store = Store::open_in_memory().unwrap();
        let err = store.update_workflow(&sample("nope")).unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[test]
    fn delete_returns_existence() {
        let store = Store::open_in_memory().unwrap();
        store.insert_workflow(&sample("w1")).unwrap();
        assert!(store.delete_workflow("w1").unwrap());
        assert!(!store.delete_workflow("w1").unwrap());
    }

    #[test]
    fn list_is_ordered() {
        let store = Store::open_in_memory().unwrap();
        store.insert_workflow(&sample("w1")).unwrap();
        store.insert_workflow(&sample("w2")).unwrap();
        let ids: Vec<String> = store.list_workflows().unwrap().into_iter().map(|w| w.id).collect();
        assert_eq!(ids, vec!["w1".to_string(), "w2".to_string()]);
    }
}
