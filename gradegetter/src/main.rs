use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use crypto_utils::{decrypt_string, encrypt_string};
use regex::Regex;
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    collections::HashMap,
    str,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::{
    net::TcpListener,
    process::Command,
    signal::{
        self,
        unix::{SignalKind, signal},
    },
};

use tracing::{debug, error, info, trace};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_string = dotenvy::var("DATABASE_URL").expect("DATABASE_URL not found");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_string)
        .await
        .context("failed to connect to database")?;

    let pool = Arc::new(pool);

    // Token Getter Thread?
    let pool_token = Arc::clone(&pool);
    tokio::spawn(async move {
        loop {
            if let Ok(users) =
                sqlx::query!("SELECT id, encrypted_email, encrypted_password FROM schoology_auth")
                    .fetch_all(&*pool_token)
                    .await
                    .map_err(|err| {
                        tracing::info!("Database error: {}", err);
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    })
            {
                for user in users {
                    let (id, email, password) =
                        (user.id, user.encrypted_email, user.encrypted_password);

                    let dec_password = match decrypt_string(password.as_str()) {
                        Ok(dec_password) => dec_password,
                        Err(err) => {
                            tracing::warn!("weird issue? {}", err);
                            "error".to_string()
                        }
                    };

                    let dec_email = match decrypt_string(email.as_str()) {
                        Ok(dec_email) => dec_email,
                        Err(err) => {
                            tracing::warn!("weird issue? {}", err);
                            "error".to_string()
                        }
                    };

                    if dec_email == "error" || dec_password == "error" {
                        error!(
                            "decyrpt_string, (token thread) skipping user due to decryption error:  {}",
                            id
                        );
                        let _ = sqlx::query!(
                            "UPDATE schoology_auth SET session_token = NULL WHERE id = $1",
                            id
                        )
                        .execute(&*pool_token)
                        .await
                        .map_err(|err| {
                            tracing::info!("Database error: {}", err);
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR
                        });

                        continue;
                    }

                    let token = match get_token(dec_email.as_str(), dec_password.as_str()).await {
                        Ok(token) => token,
                        Err(err) => {
                            tracing::error!("get_token failure with user: {}, error: {}", id, err);
                            "error".to_string()
                        }
                    };

                    if token == "error" {
                        debug!("SKIPPING USER {}", id);
                        let _ = sqlx::query!(
                            "UPDATE schoology_auth SET session_token = NULL WHERE id = $1",
                            id
                        )
                        .execute(&*pool_token)
                        .await
                        .map_err(|err| {
                            tracing::info!("Database error: {}", err);
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR
                        });

                        continue;
                    }

                    let _ = sqlx::query!(
                        "UPDATE schoology_auth SET session_token = $1 WHERE id = $2",
                        token,
                        id
                    )
                    .execute(&*pool_token)
                    .await
                    .map_err(|err| {
                        tracing::info!("Database error: {}", err);
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    });

                    info!("Updated token for UUID: {}", id);
                }
            }
            info!("Got Token!");
            tokio::time::sleep(std::time::Duration::from_secs(1800)).await // 30 minutes
        }
    });

    // Grade fetcher
    let pool_grades = Arc::clone(&pool);
    tokio::spawn(async move {
        loop {
            if let Ok(users) = sqlx::query!("SELECT id, session_token FROM schoology_auth")
                .fetch_all(&*pool_grades)
                .await
                .map_err(|err| {
                    tracing::info!("Database error: {}", err);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })
            {
                for user in users {
                    if let (id, Some(token)) = (user.id, user.session_token) {
                        let token = match decrypt_string(token.as_str()) {
                            Ok(token) => token,
                            Err(err) => {
                                tracing::warn!("weird issue? {}", err);
                                "error".to_string()
                            }
                        };

                        if token == "error" {
                            error!(
                                "decyrpt_string, (grade thread) skipping user due to decryption error:  {}",
                                id
                            );

                            let _ = sqlx::query!(
                                "UPDATE schoology_auth SET session_token = NULL WHERE id = $1",
                                id
                            )
                            .execute(&*pool_grades)
                            .await
                            .map_err(|err| {
                                tracing::info!("Database error: {}", err);
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR
                            });

                            continue;
                        }

                        match fetch_grades(token).await {
                            Ok(grades_json) => {
                                let _ = sqlx::query!(
                                "INSERT INTO grades (id, grades) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET grades = EXCLUDED.grades",
                                    id, grades_json
                                )
                                .execute(&*pool_grades)
                                .await
                                .map_err(|err| {
                                    tracing::error!("Database error: {}", err);
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                                });

                                info!("Updated grades for UUID: {}", id);
                            }
                            Err(e) => {
                                let error_msg = e.to_string();

                                tracing::warn!(
                                    "Believe to be rate limited :(((( ! Sleeping for 10 seconds..."
                                );
                                tokio::time::sleep(Duration::from_secs(10)).await;
                                tracing::error!("Failed to fetch grades for {}: {}", id, error_msg);
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await
        }
    });

    let pool_axum = Arc::clone(&pool);
    let app = Router::new()
        .route("/health", get(health))
        /*        .route(
        "/userinit",
        get({
            let pool_axum = Arc::clone(&pool_axum);
            move || {
                let pool = Arc::clone(&pool_axum);
                async move { user_token_initalize(pool).await }
            }
        }),*/
        .route(
            "/userinit",
            post({
                let pool_axum = Arc::clone(&pool_axum);
                move |uuid: String| {
                    let pool = Arc::clone(&pool_axum);
                    async move { user_token_initalize(pool, uuid).await }
                }
            }),
        );

    let listener = TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = signal::ctrl_c();

    let terminte = async {
        signal(SignalKind::terminate())
            .expect("failed to install the SIGTERM handler 🥲")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminte => {},

    }

    info!("Signal recvived now starting graceful shutdown");
}

async fn health() -> Result<(), axum::http::StatusCode> {
    Ok(())
}

async fn user_token_initalize(
    pool: Arc<PgPool>,
    uuid: String,
) -> Result<String, axum::http::StatusCode> {
    let user = sqlx::query!(
           "SELECT id, encrypted_email, encrypted_password FROM schoology_auth WHERE session_token IS NULL AND id = $1",
            uuid::Uuid::parse_str(&uuid.as_str()).unwrap()
        )
        .fetch_one(&*pool)
        .await
        .map_err(|err| {
            tracing::error!("Database error: {}", err);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        });

    let user = match user {
        Ok(user) => user,
        Err(_) => return Err(axum::http::StatusCode::UNAUTHORIZED),
    };

    let (id, email, password) = (user.id, user.encrypted_email, user.encrypted_password);

    let dec_password = decrypt_string(password.as_str());
    let dec_email = decrypt_string(email.as_str());
    let token = match get_token(dec_email.unwrap().as_str(), dec_password.unwrap().as_str()).await {
        Ok(token) => token,
        Err(err) => {
            tracing::error!("get_token failure with user: {}, error: {}", id, err);
            return Err(axum::http::StatusCode::UNAUTHORIZED);
        }
    };

    sqlx::query!(
        "UPDATE schoology_auth SET session_token = $1 WHERE id = $2",
        token,
        id
    )
    .execute(&*pool)
    .await
    .map_err(|err| {
        tracing::info!("Database error: {}", err);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("user_token_initalize: Updated token for UUID: {}", id);

    sqlx::query!("INSERT INTO grades (id, grades) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET grades = EXCLUDED.grades",
        id,
        fetch_grades(decrypt_string(token.as_str()).expect("Decrypting Token String Failed"))
            .await.map_err(|err| {
                tracing::error!("fetch_grades error: {}", err);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?,
        )
        .execute(&*pool)
        .await
        .map_err(|err| {
            tracing::error!("Database error: {}", err);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("user_token_initalize: Updated grades for UUID: {}", id);

    Ok("Hi".to_string())
}

async fn get_token(email: &str, password: &str) -> Result<String, anyhow::Error> {
    let executable =
        dotenvy::var("PUPPETEER_EXECUTABLE_PATH").expect("PUPPETEER_EXECUTABLE_PATH not found");
    let output = Command::new("node")
        .env("PUPPETEER_EXECUTABLE_PATH", executable)
        .arg("../tokengetter/") // ASSuming this is the path; adjust if needed...ass
        .arg(email)
        .arg(password)
        .output()
        .await
        .context("failed to execute tokengetter")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("the tokengetter script failed: {}", stderr);
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if token.is_empty() {
        anyhow::bail!("tokengetter returned an empty token");
    }

    if !token.starts_with("SESS") {
        anyhow::bail!(
            "tokengetter returned invalid session token format: {}",
            token
        );
    }

    Ok(encrypt_string(&token))
}

async fn fetch_grades(token: String) -> Result<Value, anyhow::Error> {
    let forms = select_grade_period(token.clone())
        .await
        .context("select_grade_period failed")?;
    let html = fetch_final_grades_export(
        forms.form_build_id.as_str(),
        forms.form_token.as_str(),
        forms.class_ids,
        token,
    )
    .await
    .context("fetch_final_grades_export failed")?;

    let grades: HashMap<String, Vec<Option<f32>>> =
        parse_grades_html(html).context("parse_grades_html failed to parse through html")?;

    Ok(serde_json::to_value(grades)?)
}

static FORM_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new(r#"name="form_build_id" id="([^"]+)""#).unwrap());

static FORM_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"<input type="hidden" name="form_token" id="edit-s-grades-export-form-form-token-1" value="([^"]+)""#).unwrap()
});

// used specifically for the select_grade_period function due to its slightly differnt html
static FORM_TOKEN_RE_GP: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new(r#"form-token" value="([^"]+)""#).unwrap());

static GRADING_PEROID_RE: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new(r#"name="grading_period\[(\d+)\]""#).unwrap());

static COURSE_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new(r#"courses\[(\d+)\]\[selected\]"#).unwrap());

#[derive(Debug)]
struct QuarterForms {
    form_build_id: String,
    form_token: String,
    grading_periods: HashMap<String, String>,
}

// this function gets the form token, form build id, and quarter information
async fn fetch_export_initial_form_data(token: String) -> Result<QuarterForms, anyhow::Error> {
    let mut form_build_id = "N/A".to_string();
    let mut form_token = "N/A".to_string();

    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Cookie", token.parse()?);
    let req = client
        .request(
            reqwest::Method::GET,
            "https://essexnorthshore.schoology.com/grades/export",
        )
        .headers(headers);

    let response = req.send().await?;
    let body = response.text().await?;

    if let Some(caps) = FORM_ID_RE.captures(body.as_str()) {
        form_build_id = caps[1].to_string();
        debug!("fetch_export_initial_form_data: form_build_id match found");
    } else {
        debug!("fetch_export_initial_form_data: form_build_id NO match found");
    }

    if let Some(caps) = FORM_TOKEN_RE.captures(body.as_str()) {
        form_token = caps[1].to_string();
        debug!("fetch_export_initial_form_data: form_token match found");
    } else {
        debug!("fetch_export_initial_form_data: form_token NO match found");
    }

    let mut grading_periods: HashMap<String, String> = HashMap::new();

    let quater_ids: Vec<String> = GRADING_PEROID_RE
        .captures_iter(&body)
        .map(|c| c.extract::<1>().1[0].to_string())
        .collect();

    for id in &quater_ids[quater_ids.len() - 4..] {
        trace!("quarter ids: {id}");
        grading_periods.insert(format!("grading_period[{}]", id), id.to_string());
    }

    let output = QuarterForms {
        form_build_id,
        form_token,
        grading_periods,
    };

    debug!("fetch_export_inital_form_data output: {:?}", output);

    Ok(output)
}

#[derive(Debug)]
struct ClassForms {
    form_build_id: String,
    form_token: String,
    class_ids: HashMap<String, String>,
}

// This inputs the form build, form id, and quarter values from last from the last function (fetch_export_initial_form_data)
// and selects the grading period
async fn select_grade_period(token: String) -> Result<ClassForms, anyhow::Error> {
    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert("Cookie", token.parse()?);

    headers.insert("accept-language", "en-US,en;q=0.9".parse()?);
    headers.insert("cache-control", "max-age=0".parse()?);
    headers.insert("content-type", "application/x-www-form-urlencoded".parse()?);
    headers.insert("origin", "https://essexnorthshore.schoology.com".parse()?);
    headers.insert("priority", "u=0, i".parse()?);
    headers.insert(
        "referer",
        "https://essexnorthshore.schoology.com/grades/grades".parse()?,
    );
    headers.insert(
        "sec-ch-ua",
        "\"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"".parse()?,
    );
    headers.insert("sec-ch-ua-mobile", "?0".parse()?);
    headers.insert("sec-ch-ua-platform", "\"macOS\"".parse()?);
    headers.insert("sec-fetch-dest", "document".parse()?);
    headers.insert("sec-fetch-mode", "navigate".parse()?);
    headers.insert("sec-fetch-site", "same-origin".parse()?);
    headers.insert("sec-fetch-user", "?1".parse()?);
    headers.insert("upgrade-insecure-requests", "1".parse()?);

    let params_needed = fetch_export_initial_form_data(token)
        .await
        .context("fetch_export_form_tokens failed")?;

    let mut params = params_needed.grading_periods;

    params.insert("form_id".to_string(), "s_grades_export_form".to_string());
    params.insert("op".to_string(), "Next".to_string());
    params.insert("form_build_id".to_string(), params_needed.form_build_id);
    params.insert("form_token".to_string(), params_needed.form_token);

    let req = client
        .request(
            reqwest::Method::POST,
            "https://essexnorthshore.schoology.com/grades/export",
        )
        .headers(headers)
        .form(&params);

    let response = req.send().await?;
    let body = response.text().await?;

    let mut form_build_id = "N/A".to_string();
    let mut form_token = "N/A".to_string();

    if let Some(caps) = FORM_ID_RE.captures(body.as_str()) {
        form_build_id = caps[1].to_string();
        debug!("select_grade_period: form_build_id match found");
    } else {
        debug!("select_grade_period: form_build_id NO match found");
    }

    if let Some(caps) = FORM_TOKEN_RE_GP.captures(body.as_str()) {
        form_token = caps[1].to_string();
        debug!("select_grade_period: form_token match found");
    } else {
        debug!("select_grade_period: form_token NO match found");
    }

    let class_ids = fetch_class_ids(body)
        .await
        .context("fetch_class_ids failed: {}")?;

    let output = ClassForms {
        form_build_id,
        form_token,
        class_ids,
    };

    debug!("select_grade_period output: {:?}", output);

    Ok(output)
}

async fn fetch_class_ids(body: String) -> Result<HashMap<String, String>, anyhow::Error> {
    let mut hashmap: HashMap<String, String> = HashMap::new();

    for (_, [id]) in COURSE_ID_RE
        .captures_iter(body.as_str())
        .map(|c| c.extract())
    {
        trace!("class ids: {id}");
        hashmap.insert(format!("courses[{}][selected]", id), "1".to_string());
    }
    Ok(hashmap)
}

type ClassIdsHashMap = HashMap<String, String>;

// Selects classes and gets the final export, creates html file
async fn fetch_final_grades_export(
    form_build_id: &str,
    form_token: &str,
    class_ids_hashmap: ClassIdsHashMap,
    token: String,
) -> Result<String, anyhow::Error> {
    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert("Cookie", token.parse()?);

    headers.insert("accept-language", "en-US,en;q=0.9".parse()?);
    headers.insert("cache-control", "max-age=0".parse()?);
    headers.insert("content-type", "application/x-www-form-urlencoded".parse()?);
    headers.insert("origin", "https://essexnorthshore.schoology.com".parse()?);
    headers.insert("priority", "u=0, i".parse()?);
    headers.insert(
        "referer",
        "https://essexnorthshore.schoology.com/grades/grades".parse()?,
    );
    headers.insert(
        "sec-ch-ua",
        "\"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"".parse()?,
    );
    headers.insert("sec-ch-ua-mobile", "?0".parse()?);
    headers.insert("sec-ch-ua-platform", "\"macOS\"".parse()?);
    headers.insert("sec-fetch-dest", "document".parse()?);
    headers.insert("sec-fetch-mode", "navigate".parse()?);
    headers.insert("sec-fetch-site", "same-origin".parse()?);
    headers.insert("sec-fetch-user", "?1".parse()?);
    headers.insert("upgrade-insecure-requests", "1".parse()?);

    let mut params = class_ids_hashmap;

    params.insert("form_id".to_string(), "s_grades_export_form".to_string());
    params.insert("form_build_id".to_string(), form_build_id.to_string());
    params.insert("form_token".to_string(), form_token.to_string());

    let req = client
        .request(
            reqwest::Method::POST,
            "https://essexnorthshore.schoology.com/grades/export",
        )
        .headers(headers)
        .form(&params);
    let response = req.send().await?;

    debug!("fetch_final_grades_export: page_url {}", response.url());

    let body = response.text().await?;

    Ok(body)
}

type GradesHashMap = HashMap<String, Vec<Option<f32>>>;

fn parse_grades_html(html: String) -> Result<GradesHashMap, anyhow::Error> {
    let document = scraper::Html::parse_document(html.as_str());
    let grade_selector = scraper::Selector::parse("td.grade, td.grade.no-grade")
        .expect("could not parse grade_selector");

    let row_selector = scraper::Selector::parse("tr").expect("could not parse row_selector");

    let mut course_grades: GradesHashMap = HashMap::new();
    let mut current_course: Option<String> = None;

    for row in document.select(&row_selector) {
        if row.value().has_class(
            "course-title",
            scraper::CaseSensitivity::AsciiCaseInsensitive,
        ) {
            let title_text = row.text().collect::<String>().trim().to_string();
            let cleaned_title_text: String = match title_text.find("\u{a0}:\u{a0}") {
                Some(index) => title_text[..index].to_string(),
                None => title_text.to_string(),
            };

            if cleaned_title_text == "Class of 2028 Guidance" {
                continue;
            }

            current_course = Some(cleaned_title_text.clone());
            course_grades.insert(cleaned_title_text, Vec::new());
        } else if let Some(course) = &current_course {
            for grade_cell in row.select(&grade_selector) {
                if Some("grade final-grade") == grade_cell.attr("class") {
                    // dont add "final-grade" in the course_grades hashmap
                    continue;
                };

                let grade_text = grade_cell
                    .text()
                    .collect::<String>()
                    .trim()
                    .replace("%", "");
                let grade = grade_text.parse::<f32>().ok();
                course_grades.get_mut(course).unwrap().push(grade);
            }
        }
    }

    Ok(course_grades)
}
