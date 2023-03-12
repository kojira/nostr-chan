use rusqlite::{params, Connection, Result};

pub(crate) fn connect() -> Result<Connection> {
    let conn = Connection::open("../nostrchan.db")?;
    Ok(conn)
}

#[derive(Debug)]
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

fn get_person(conn: &Connection) -> Result<Person> {
    let mut stmt = conn.prepare("SELECT * FROM Persons LIMIT 1")?;
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

    Ok(person)
}

pub fn get_random_person(conn: &Connection) -> Result<Person> {
    let mut stmt = conn.prepare("SELECT * FROM Persons ORDER BY RANDOM() LIMIT 1")?;
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

    Ok(person)
}
