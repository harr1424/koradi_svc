use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpResponse, HttpServer};
use chrono::prelude::*;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use reqwest;
use serde_derive::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::read_to_string,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;

const REFRESH_HASH_IN_SECONDS: u64 = 60;

#[derive(Debug, Deserialize)]
struct Config {
    secrets: Secrets,
}

#[derive(Debug, Deserialize)]
struct Secrets {
    en_image: String,
    en_image_p: String,
    es_image: String,
    es_image_p: String,
    fr_image: String,
    po_image: String,
    it_image: String,
    de_image: String,
}

struct AppState {
    en_image_hash: Mutex<String>,
    en_p_image_hash: Mutex<String>,
    es_image_hash: Mutex<String>,
    es_p_image_hash: Mutex<String>,
    fr_image_hash: Mutex<String>,
    po_image_hash: Mutex<String>,
    it_image_hash: Mutex<String>,
    de_image_hash: Mutex<String>,
    notify: Notify,
}

impl Config {
    fn load_from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = read_to_string(filename)
            .map_err(|err| format!("Unable to read config file: {}", err))?;
        let config: Config = toml::from_str(&config_str)
            .map_err(|err| format!("Unable to parse config file: {}", err))?;
        Ok(config)
    }
}

macro_rules! download_and_hash_image {
    ($state_mu:expr, $image:expr, $state_notify:expr) => {
        let image_data = match reqwest::get($image).await {
            Ok(response) => match response.bytes().await {
                Ok(data) => data,
                Err(e) => {
                    let now: DateTime<Utc> = Utc::now();
                    eprintln!("{} : Error reading response bytes: {}", now, e);
                    continue;
                }
            },
            Err(e) => {
                let now: DateTime<Utc> = Utc::now();
                eprintln!("{} : Error fetching image: {}", now, e);
                continue;
            }
        };
        let hash = format!("{:x}", Sha256::digest(&image_data));
        {
            let mut image_hash = $state_mu.lock().unwrap();
            *image_hash = hash.clone();
            $state_notify.notify_one();
        }
    };
}
async fn download_and_hash_images(state: Arc<AppState>, config: Config) {
    let en_image = config.secrets.en_image.clone();
    let en_p_image = config.secrets.en_image_p.clone();
    let es_image = config.secrets.es_image.clone();
    let es_p_image = config.secrets.es_image_p.clone();
    let fr_image = config.secrets.fr_image.clone();
    let po_image = config.secrets.po_image.clone();
    let it_image = config.secrets.it_image.clone();
    let de_image = config.secrets.de_image.clone();

    loop {
        download_and_hash_image!(state.en_image_hash, &en_image, state.notify);
        download_and_hash_image!(state.en_p_image_hash, &en_p_image, state.notify);
        download_and_hash_image!(state.es_image_hash, &es_image, state.notify);
        download_and_hash_image!(state.es_p_image_hash, &es_p_image, state.notify);
        download_and_hash_image!(state.fr_image_hash, &fr_image, state.notify);
        download_and_hash_image!(state.po_image_hash, &po_image, state.notify);
        download_and_hash_image!(state.it_image_hash, &it_image, state.notify);
        download_and_hash_image!(state.de_image_hash, &de_image, state.notify);

        tokio::time::sleep(Duration::from_secs(REFRESH_HASH_IN_SECONDS)).await;
    }
}

macro_rules! create_hash_endpoint {
    ($state_field:ident, $route:expr) => {
        #[get($route)]
        async fn $state_field(state: web::Data<Arc<AppState>>) -> HttpResponse {
            let image_hash = state.$state_field.lock().unwrap();
            HttpResponse::Ok().body(image_hash.clone())
        }
    };
}

create_hash_endpoint!(en_image_hash, "/en");
create_hash_endpoint!(en_p_image_hash, "/en_p");
create_hash_endpoint!(es_image_hash, "/es");
create_hash_endpoint!(es_p_image_hash, "/es_p");
create_hash_endpoint!(fr_image_hash, "/fr");
create_hash_endpoint!(po_image_hash, "/po");
create_hash_endpoint!(it_image_hash, "/it");
create_hash_endpoint!(de_image_hash, "/de");


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = Arc::new(AppState {
        en_image_hash: Mutex::new(String::new()),
        en_p_image_hash: Mutex::new(String::new()),
        es_image_hash: Mutex::new(String::new()),
        es_p_image_hash: Mutex::new(String::new()),
        fr_image_hash: Mutex::new(String::new()),
        po_image_hash: Mutex::new(String::new()),
        it_image_hash: Mutex::new(String::new()),
        de_image_hash: Mutex::new(String::new()),
        notify: Notify::new(),
    });

    let app_state_clone = Arc::clone(&app_state);

    tokio::spawn(async move {
        let config = Config::load_from_file("Config.toml").unwrap_or_else(|e| {
            eprintln!("Error loading config: {}", e);
            std::process::exit(1);
        });

        download_and_hash_images(app_state_clone, config).await;
    });

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("certs/key.pem", SslFiletype::PEM)
        .unwrap();
    builder
        .set_certificate_chain_file("certs/cert.pem")
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(Logger::default())
            .service(en_image_hash)
            .service(en_p_image_hash)
            .service(es_image_hash)
            .service(es_p_image_hash)
            .service(fr_image_hash)
            .service(po_image_hash)
            .service(it_image_hash)
            .service(de_image_hash)
    })
    .bind_openssl("0.0.0.0:9191", builder)?
    .run()
    .await
}
