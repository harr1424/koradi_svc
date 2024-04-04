use std::{
    sync::{Arc, Mutex},
    time::Duration,
    fs::read_to_string,
};
use actix_web::{get, web, App, HttpServer, HttpResponse};
use reqwest;
use sha2::{Digest, Sha256};
use serde_derive::Deserialize;
use tokio::{
    sync::Notify,
};

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
                    eprintln!("Error reading response bytes: {}", e);
                    continue; 
                }
            },
            Err(e) => {
                eprintln!("Error fetching image: {}", e);
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

#[get("/en")]
async fn get_en_hash(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let image_hash = state.en_image_hash.lock().unwrap();
    HttpResponse::Ok().body(image_hash.clone())
}

#[get("/en_p")]
async fn get_en_p_hash(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let image_hash = state.en_p_image_hash.lock().unwrap();
    HttpResponse::Ok().body(image_hash.clone())
}

#[get("/es")]
async fn get_es_hash(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let image_hash = state.es_image_hash.lock().unwrap();
    HttpResponse::Ok().body(image_hash.clone())
}

#[get("/es_p")]
async fn get_es_p_hash(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let image_hash = state.es_p_image_hash.lock().unwrap();
    HttpResponse::Ok().body(image_hash.clone())
}

#[get("/fr")]
async fn get_fr_hash(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let image_hash = state.fr_image_hash.lock().unwrap();
    HttpResponse::Ok().body(image_hash.clone())
}

#[get("/po")]
async fn get_po_hash(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let image_hash = state.po_image_hash.lock().unwrap();
    HttpResponse::Ok().body(image_hash.clone())
}

#[get("/it")]
async fn get_it_hash(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let image_hash = state.it_image_hash.lock().unwrap();
    HttpResponse::Ok().body(image_hash.clone())
}

#[get("/de")]
async fn get_de_hash(state: web::Data<Arc<AppState>>) -> HttpResponse {
    let image_hash = state.de_image_hash.lock().unwrap();
    HttpResponse::Ok().body(image_hash.clone())
}

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

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(get_en_hash)
            .service(get_en_p_hash)
            .service(get_es_hash)
            .service(get_es_p_hash)
            .service(get_fr_hash)
            .service(get_po_hash)
            .service(get_it_hash)
            .service(get_de_hash)
        })
        .bind("127.0.0.1:9191")?
        .run()
        .await
}
