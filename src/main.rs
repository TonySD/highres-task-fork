use std::sync::Mutex;

use actix_web::{web, App, HttpServer};
use db::DbPool;
use log::info;
use num_bigint_dig::{IntoBigUint, ModInverse, RandBigInt, RandPrime};
use num_integer::Integer;
use rand_core::CryptoRngCore;
use rsa::{BigUint, RsaPrivateKey};

mod db;
mod encryption;
mod server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
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

    info!("Starting service...");
    let pool = DbPool::new().await.expect("failed to establish connection");
    let pool_mtx = web::Data::new(Mutex::new(pool));
    info!("Established a connection to the database");

    HttpServer::new(move || {
        App::new()
            .service(server::hello)
            .service(server::get_token)
            .service(server::get_note)
            .service(server::get_key)
            .service(server::add_note)
            .app_data(pool_mtx.clone())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
    // let mut rng = rand::thread_rng();
}
