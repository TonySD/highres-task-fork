use std::time::Duration;

use actix_web::cookie::time::error::InvalidVariant;
use anyhow::Result;
use log::info;
use rsa::{traits::PublicKeyParts, RsaPublicKey};
use sqlx::PgPool;

use crate::server::ServerError;

#[derive(Debug, Clone)]
pub struct DbPool(PgPool);

#[derive(Clone)]
pub struct NoteRow {
    pub id: i32,
    pub contents: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct KeyRow {
    id: i32,
    pub n: Option<Vec<u8>>,
    pub e: Option<Vec<u8>>,
}

impl DbPool {
    /// Establishes a new connection to the postgres database and calls on [`create_db`]
    /// To create the relevant table
    pub async fn new() -> Result<Self> {
        info!("Trying to establish a connection to the database");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect("postgres://postgres:password@db/postgres")
            .await?;
        let res = Self(pool);
        info!("Connection established");
        res.create_databases().await?;
        Ok(res)
    }

    pub async fn fetch_notes(&self) -> Result<Vec<NoteRow>, sqlx::Error> {
        sqlx::query_as!(NoteRow, "SELECT id, contents FROM notes")
            .fetch_all(&self.0)
            .await
    }

    pub async fn fetch_keys(&self) -> Result<Vec<KeyRow>, sqlx::Error> {
        sqlx::query_as!(KeyRow, "SELECT id, n, e FROM publickeys")
            .fetch_all(&self.0)
            .await
    }

    pub async fn get_note(&self, note_id: u32) -> Result<NoteRow, ServerError> {
        let rows = self.fetch_notes().await?;
        let res = rows
            .iter()
            .find(|NoteRow { id, .. }| *id as u32 == note_id)
            .ok_or_else(|| ServerError::LookupError)?;
        Ok(res.clone())
    }

    pub async fn get_key(&self, key_id: u32) -> Result<KeyRow, ServerError> {
        let rows = self.fetch_keys().await?;
        let res = rows
            .iter()
            .find(|KeyRow { id, .. }| *id as u32 == key_id)
            .ok_or_else(|| ServerError::LookupError)?;
        Ok(res.clone())
    }

    pub async fn add_key(&self, pub_key: &RsaPublicKey) -> Result<(), ServerError> {
        info!("Adding key");
        sqlx::query!(
            r#"
            INSERT INTO publickeys (n, e) VALUES ($1, $2)
        "#,
            pub_key.n().to_bytes_le(),
            pub_key.e().to_bytes_le()
        )
        .execute(&self.0)
        .await?;
        info!("Key added");

        Ok(())
    }

    pub async fn add_note_enc(&self, msg: Vec<u8>) -> Result<(), ServerError> {
        info!("Adding encrypted note");
        sqlx::query!("INSERT INTO notes (contents) VALUES ($1)", msg)
            .execute(&self.0)
            .await?;
        info!("Note added");
        Ok(())
    }

    async fn create_notes_db(&self) -> Result<()> {
        info!("Creating notes db");
        sqlx::query!(
            r#"
                        CREATE TABLE IF NOT EXISTS notes (
                            id serial primary key,
                            contents bytea
                        );
                    "#,
        )
        .execute(&self.0)
        .await?;
        info!("Notes db created");
        Ok(())
    }

    async fn create_publickeys_db(&self) -> Result<()> {
        info!("Creating publickeys db");
        sqlx::query!(
            r#"
                    CREATE TABLE IF NOT EXISTS publickeys (
                        id serial primary key,
                        n bytea,
                        e bytea
                    );
                "#,
        )
        .execute(&self.0)
        .await?;
        info!("Publickeys db created");
        Ok(())
    }

    /// Creates the table via the existing valid postgres connection
    /// You shouldn't have to call this manually if you're using [`new`]
    pub async fn create_databases(&self) -> Result<()> {
        info!("Creating databases");
        // Create the table for notes
        self.create_notes_db().await?;
        self.create_publickeys_db().await?;
        info!("All databases created");
        Ok(())
    }
}
