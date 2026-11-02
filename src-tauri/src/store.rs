use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub text: String,
    pub notes: String,
    pub summary: String,
    pub created: String,
    pub duration: f64,
    pub status: String,
    pub audio: String,
    pub segments: String,
}
pub fn open(root: &Path) -> Result<Connection, String> {
    let c = Connection::open(root.join("quicktalk.sqlite")).map_err(|e| e.to_string())?;
    c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000; CREATE TABLE IF NOT EXISTS entries(id TEXT PRIMARY KEY,kind TEXT NOT NULL,title TEXT NOT NULL,text TEXT NOT NULL DEFAULT '',notes TEXT NOT NULL DEFAULT '',summary TEXT NOT NULL DEFAULT '',created TEXT NOT NULL,duration REAL NOT NULL DEFAULT 0,status TEXT NOT NULL,audio TEXT NOT NULL,segments TEXT NOT NULL DEFAULT '[]'); CREATE VIRTUAL TABLE IF NOT EXISTS search USING fts5(id UNINDEXED,title,text,notes); CREATE TRIGGER IF NOT EXISTS entries_insert AFTER INSERT ON entries BEGIN INSERT INTO search(id,title,text,notes) VALUES(new.id,new.title,new.text,new.notes); END; CREATE TRIGGER IF NOT EXISTS entries_delete AFTER DELETE ON entries BEGIN DELETE FROM search WHERE id=old.id; END; CREATE TRIGGER IF NOT EXISTS entries_update AFTER UPDATE ON entries BEGIN DELETE FROM search WHERE id=old.id; INSERT INTO search(id,title,text,notes) VALUES(new.id,new.title,new.text,new.notes); END;").map_err(|e|e.to_string())?;
    Ok(c)
}
pub fn save(root: &Path, e: &Entry) -> Result<(), String> {
    open(root)?.execute("INSERT INTO entries VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(id) DO UPDATE SET title=excluded.title,text=excluded.text,notes=excluded.notes,summary=excluded.summary,duration=excluded.duration,status=excluded.status,segments=excluded.segments",params![e.id,e.kind,e.title,e.text,e.notes,e.summary,e.created,e.duration,e.status,e.audio,e.segments]).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn list(root: &Path) -> Result<Vec<Entry>, String> {
    let c = open(root)?;
    let mut s = c
        .prepare("SELECT * FROM entries ORDER BY created DESC")
        .map_err(|e| e.to_string())?;
    let rows = s
        .query_map([], |r| {
            Ok(Entry {
                id: r.get(0)?,
                kind: r.get(1)?,
                title: r.get(2)?,
                text: r.get(3)?,
                notes: r.get(4)?,
                summary: r.get(5)?,
                created: r.get(6)?,
                duration: r.get(7)?,
                status: r.get(8)?,
                audio: r.get(9)?,
                segments: r.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}
pub fn get(root: &Path, id: &str) -> Result<Entry, String> {
    list(root)?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or("Recording not found".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stores_unicode_and_updates_search() {
        let p = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&p).unwrap();
        let mut e = Entry {
            id: "1".into(),
            kind: "dictation".into(),
            title: "日本語".into(),
            text: "مرحبا wereld".into(),
            notes: String::new(),
            summary: String::new(),
            created: "2026-01-01".into(),
            duration: 1.,
            status: "ready".into(),
            audio: String::new(),
            segments: "[]".into(),
        };
        save(&p, &e).unwrap();
        assert_eq!(get(&p, "1").unwrap().text, e.text);
        e.text = "updated".into();
        save(&p, &e).unwrap();
        assert_eq!(list(&p).unwrap().len(), 1);
        assert_eq!(
            open(&p)
                .unwrap()
                .query_row(
                    "SELECT count(*) FROM search WHERE search MATCH 'updated'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        std::fs::remove_dir_all(p).unwrap();
    }
}
