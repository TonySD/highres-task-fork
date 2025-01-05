use actix_web::{App, HttpServer};
use db::DbPool;
use num_bigint_dig::{IntoBigUint, ModInverse, RandBigInt, RandPrime};
use num_integer::Integer;
use rand_core::CryptoRngCore;
use rsa::{BigUint, RsaPrivateKey};

mod encryption;

mod db {
    use std::time::Duration;

    use anyhow::{anyhow, Result};
    use sqlx::{prelude::FromRow, PgPool};

    #[derive(Debug, Clone)]
    pub struct DbPool(PgPool);

    #[derive(FromRow, Clone)]
    pub struct NoteRow {
        id: i32,
        contents: Vec<u8>,
    }

    impl DbPool {
        /// Establishes a new connection to the postgres database and calls on [`create_db`]
        /// To create the relevant table
        pub async fn new() -> Result<Self> {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(Duration::from_secs(5))
                .connect("postgres://postgres:password@db/postgres")
                .await?;
            let res = Self(pool);
            res.create_db().await?;
            Ok(res)
        }

        pub async fn fetch_notes(&self) -> Result<Vec<NoteRow>, sqlx::Error> {
            let rows = sqlx::query("SELECT * FROM notes")
                .fetch_all(&self.0)
                .await?
                .into_iter()
                .map(|r| NoteRow::from_row(&r))
                .collect::<Vec<_>>();

            if rows.iter().all(Result::is_ok) {
                Ok(rows.into_iter().map(Result::unwrap).collect::<Vec<_>>())
            } else {
                rows.into_iter()
                    .reduce(|acc, e| match (&acc, &e) {
                        (Err(_), _) => acc,
                        (Ok(_), Err(_)) => e,
                        _ => acc,
                    })
                    .expect("Should always work")
                    .map(|r| vec![r])
            }
        }

        pub async fn get_note(&self, note_id: i32) -> Result<NoteRow> {
            let rows = self.fetch_notes().await?;
            let res = rows
                .iter()
                .find(|NoteRow { id, .. }| id == &note_id)
                .ok_or(anyhow!("Failed to find note by id {}", note_id));
            res.cloned()
        }

        /// Creates the table via the existing valid postgres connection
        /// You shouldn't have to call this manually if you're using [`new`]
        pub async fn create_db(&self) -> Result<()> {
            // Create table
            sqlx::query(
                r#"
                        CREATE TABLE IF NOT EXISTS notes (
                            id bigserial,
                            contents bytea
                        );
                    "#,
            )
            .execute(&self.0)
            .await?;
            Ok(())
        }
    }
}

mod server {
    use actix_web::{get, post, web, HttpResponse, Responder};

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct PlaintextNote {
        contents: String,
        token: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct EncryptedNote {
        contents: Vec<u8>,
    }

    #[derive(Serialize, Deserialize)]
    pub enum Note {
        Encrypted(EncryptedNote),
        Plaintext(PlaintextNote),
    }

    impl From<EncryptedNote> for Note {
        fn from(value: EncryptedNote) -> Self {
            Self::Encrypted(value)
        }
    }

    impl From<PlaintextNote> for Note {
        fn from(value: PlaintextNote) -> Self {
            Self::Plaintext(value)
        }
    }

    #[get("/")]
    async fn hello() -> impl Responder {
        HttpResponse::Ok().body("Best service everrr!")
    }

    #[post("/add_note")]
    async fn add_note(_note: web::Json<PlaintextNote>) -> impl Responder {
        HttpResponse::Ok().body("Note saved")
    }

    #[post("/note/{note_id}")]
    async fn get_note(path: web::Path<u32>) -> impl Responder {
        let note_id = path.into_inner();

        // Get note from DB

        HttpResponse::Ok().body(format!("You for the note {}", note_id))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // let mut rng = rand::thread_rng();
    // let priv_key =
    //     MyAwesomeRsaGenerator::new(&mut rng, 1024).expect("failed to create rsa generator");
    // let pub_key = RsaPublicKey::new_unchecked(priv_key.n().clone(), priv_key.e().clone());

    // let data = b"hello world!";

    // let enc_data = pub_key
    //     .encrypt(&mut rng, Pkcs1v15Encrypt, &data[..])
    //     .expect("failed to encrypt");
    // assert_ne!(&data[..], &enc_data[..]);

    // let dec_data = priv_key
    //     .decrypt(Pkcs1v15Encrypt, &enc_data)
    //     .expect("Failed to decrypt");
    // assert_eq!(&data[..], &dec_data[..]);
    //

    let pool = DbPool::new().await.expect("failed to establish connection");
    HttpServer::new(move || App::new().service(server::hello).app_data(pool.clone()))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
    // let mut rng = rand::thread_rng();
}
