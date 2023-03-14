use nostr_sdk::prelude::*;
use rusqlite::{params, Connection, Result};

pub(crate) fn connect() -> Result<Connection> {
    let conn = Connection::open("../nostrchan.db")?;
    Ok(conn)
}

#[derive(Debug, Clone)]
pub struct Person {
    pub id: i32,
    pub status: i32,
    pub prompt: String,
    pub pubkey: String,
    pub secretkey: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn get_all_persons(conn: &Connection) -> Result<Vec<Person>> {
    let mut stmt = conn.prepare("SELECT * FROM Persons WHERE status=0")?;
    let persons = stmt
        .query_map(params![], |row| {
            Ok(Person {
                id: row.get(0)?,
                status: row.get(1)?,
                prompt: row.get(2)?,
                pubkey: row.get(3)?,
                secretkey: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<Person>, _>>()?;

    Ok(persons.clone())
}

pub fn get_person(conn: &Connection, pubkey: &str) -> Result<Person> {
    let mut stmt = conn.prepare("SELECT * FROM Persons WHERE pubkey = ?")?;
    let person = stmt
        .query_map(params![pubkey], |row| {
            Ok(Person {
                id: row.get(0)?,
                status: row.get(1)?,
                prompt: row.get(2)?,
                pubkey: row.get(3)?,
                secretkey: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .next()
        .unwrap()?;

    Ok(person.clone())
}

pub(crate) fn insert_person(
    conn: &Connection,
    keys: &Keys,
    prompt: &str,
    content: &str,
) -> Result<()> {
    let secretkey = keys.secret_key().unwrap().display_secret().to_string();
    let mut stmt = conn.prepare(
        "INSERT INTO Persons (status, prompt, pubkey, secretkey, content) VALUES(0,?,?,?,?)",
    )?;
    stmt.execute(params![
        prompt,
        keys.public_key().to_string(),
        secretkey,
        content
    ])?;

    Ok(())
}

pub(crate) fn update_person_content(conn: &Connection, pubkey: &str, content: &str) -> Result<()> {
    let mut stmt = conn.prepare("UPDATE Persons SET content=? WHERE pubkey=?")?;
    stmt.execute(params![content, pubkey])?;

    Ok(())
}

pub fn get_random_person(conn: &Connection) -> Result<Person> {
    let mut stmt =
        conn.prepare("SELECT * FROM Persons WHERE status=0 ORDER BY RANDOM() LIMIT 1")?;
    let person = stmt
        .query_map(params![], |row| {
            Ok(Person {
                id: row.get(0)?,
                status: row.get(1)?,
                prompt: row.get(2)?,
                pubkey: row.get(3)?,
                secretkey: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .next()
        .unwrap()?;

    Ok(person.clone())
}
