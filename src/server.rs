use std::sync::Mutex;

use actix_web::{get, post, web, HttpResponse, Responder, ResponseError};

use base64::prelude::*;
use log::info;
use rsa::{
    traits::{PrivateKeyParts, PublicKeyParts},
    BigUint, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{db::DbPool, encryption::AwesomeRsaGenerator};

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("failed to generate key")]
    Keygen(#[from] rsa::Error),
    #[error("failed to perform a DB operation")]
    DbError(#[from] sqlx::Error),
    #[error("failed to decode b64 token")]
    B64Error(#[from] base64::DecodeError),
    #[error("wrong token format")]
    FormatError,
    #[error("Failed to find note with that id")]
    LookupError,
}

impl ResponseError for ServerError {}

#[derive(Serialize, Deserialize)]
pub struct PlaintextNote {
    /// The contents of the note
    contents: String,
    /// b64 encoded string that contains the private d key
    token: String,
}

#[derive(Deserialize)]
pub struct Token {
    token: String,
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Best service everrr!")
}

#[post("/add_note")]
async fn add_note(
    note: web::Json<PlaintextNote>,
    pool: web::Data<Mutex<DbPool>>,
) -> impl Responder {
    let components = note.token.split(':').collect::<Vec<_>>();

    if components.len() < 5 {
        return Err(ServerError::FormatError);
    }

    let process_val = |v: &str| -> Result<BigUint, ServerError> {
        let vec = BASE64_STANDARD.decode(v)?;
        let biguint = BigUint::from_bytes_le(vec.as_slice());
        Ok(biguint)
    };

    let d = process_val(components[0])?;
    let e = process_val(components[1])?;
    let n = process_val(components[2])?;
    let primes = components[5..]
        .iter()
        .map(|s: &&str| process_val(*s))
        .collect::<Result<Vec<_>, _>>()?;

    let msg = note.contents.clone();
    let priv_key = RsaPrivateKey::from_components(n, e, d, primes)?;
    let pub_key = priv_key.to_public_key();
    let mut rng = rand::thread_rng();
    let msg_enc = pub_key.encrypt(&mut rng, Pkcs1v15Encrypt, msg.as_bytes())?;

    pool.lock()
        .expect("Should lock ok")
        .add_note_enc(msg_enc)
        .await?;

    Ok(HttpResponse::Ok().body("Note saved"))
}

#[get("/keys/{key_id}")]
async fn get_key(
    path: web::Path<u32>,
    pool: web::Data<Mutex<DbPool>>,
) -> Result<impl Responder, ServerError> {
    let key_id = path.into_inner();
    let key = pool.lock().expect("Should lock ok").get_key(key_id).await?;
    let n_b64 = BASE64_STANDARD.encode(key.n.expect("Should be non empty"));
    let e_b64 = BASE64_STANDARD.encode(key.e.expect("Should be non empty"));
    Ok(HttpResponse::Ok().body(format!("n: {}\ne: {}", n_b64, e_b64)))
}

#[get("/note/{note_id}")]
async fn get_note(
    path: web::Path<u32>,
    pool: web::Data<Mutex<DbPool>>,
    token: web::Query<Token>,
) -> Result<impl Responder, ServerError> {
    let note_id = path.into_inner();
    let components = token.token.split(':').collect::<Vec<_>>();

    if components.len() < 5 {
        return Err(ServerError::FormatError);
    }

    let process_val = |v: &str| -> Result<BigUint, ServerError> {
        let vec = BASE64_STANDARD.decode(v)?;
        let biguint = BigUint::from_bytes_le(vec.as_slice());
        Ok(biguint)
    };

    let d = process_val(components[0])?;
    let e = process_val(components[1])?;
    let n = process_val(components[2])?;
    let primes = components[5..]
        .iter()
        .map(|s: &&str| process_val(*s))
        .collect::<Result<Vec<_>, _>>()?;

    // Get note from DB
    let message_enc = pool
        .lock()
        .expect("Should lock ok")
        .get_note(note_id)
        .await?;

    let priv_key = RsaPrivateKey::from_components(n, e, d, primes)?;
    let msg = priv_key.decrypt(
        Pkcs1v15Encrypt,
        message_enc
            .contents
            .expect("Should not be empty")
            .as_slice(),
    )?;

    Ok(HttpResponse::Ok().body(format!(
        "{}",
        String::from_utf8(msg).expect("Should convert ok")
    )))
}

#[get("/get_token")]
async fn get_token(pool: web::Data<Mutex<DbPool>>) -> Result<impl Responder, ServerError> {
    info!("Trying to get token");
    let mut rng = rand::thread_rng();
    let priv_key =
        AwesomeRsaGenerator::new(&mut rng, 1024).expect("failed to create rsa generator");
    let pub_key = RsaPublicKey::new_unchecked(priv_key.n().clone(), priv_key.e().clone());

    pool.lock()
        .expect("Should lock mutex")
        .add_key(&pub_key)
        .await?;

    let d_b64 = BASE64_STANDARD.encode(priv_key.d().to_bytes_le());
    let e_b64 = BASE64_STANDARD.encode(pub_key.e().to_bytes_le());
    let n_b64 = BASE64_STANDARD.encode(pub_key.n().to_bytes_le());
    let primes_b64 = priv_key
        .primes()
        .into_iter()
        .map(|n| BASE64_STANDARD.encode(n.to_bytes_le()))
        .collect::<Vec<_>>()
        .join(":");
    Ok(HttpResponse::Ok().body(format!(
        "Token {}:{}:{}:{}",
        d_b64, e_b64, n_b64, primes_b64
    )))
}
