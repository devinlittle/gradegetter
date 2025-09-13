use axum::{Router, response::IntoResponse, routing::get};
use crypto_utils::{decrypt_string, encrypt_string};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, str, sync::Arc, time::Duration};
use tokio::{net::TcpListener, process::Command};
use tracing::{debug, info, trace};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_string = dotenvy::var("DATABASE_URL").expect("DATABASE_URL not found");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_string)
        .await
        .expect("can't connect to database");
    let pool = Arc::new(pool);

    // Token Getter Thread?
    let pool_token = Arc::clone(&pool);
    tokio::spawn(async move {
        loop {
            if let Ok(users) =
                sqlx::query!("SELECT id, encrypted_email, encrypted_password FROM schoology_auth")
                    .fetch_all(&*pool_token)
                    .await
            {
                for user in users {
                    let (id, email, password) =
                        (user.id, user.encrypted_email, user.encrypted_password);

                    let dec_password = decrypt_string(password.as_str());
                    let dec_email = decrypt_string(email.as_str());

                    let _ = sqlx::query!(
                        "UPDATE schoology_auth SET session_token = $1 WHERE id = $2",
                        get_token(dec_email.unwrap().as_str(), dec_password.unwrap().as_str())
                            .await
                            .unwrap(),
                        id
                    )
                    .execute(&*pool_token)
                    .await;
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
            {
                for user in users {
                    if let (id, Some(token)) = (user.id, user.session_token) {
                        let _ = sqlx::query!(
                                "INSERT INTO grades (id, grades) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET grades = EXCLUDED.grades",
                                id,
                                fetch_grades(
                                    decrypt_string(token.as_str()).expect("Decrypting Token String Failed")
                                )
                                .await
                                .unwrap()
                            )
                            .execute(&*pool_grades)
                            .await
                            .map_err(|err| {
                                tracing::error!("Database error: {}", err);
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR
                            });
                        info!("Updated grades for UUID: {}", id);
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await
        }
    });

    let app = Router::new().route("/", get(health));
    let listener = TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> impl IntoResponse {
    "Alive".to_string()
}

async fn get_token(email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("node")
        .arg("../tokengetter/") // Assuming this is the path; adjust if needed
        .arg(email)
        .arg(password)
        .output()
        .await?;

    Ok(encrypt_string(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

async fn fetch_grades(token: String) -> Result<Value, Box<dyn std::error::Error>> {
    let forms = select_grade_period(token.clone()).await?;
    let html = fetch_final_grades_export(
        forms.form_build_id.as_str(),
        forms.form_token.as_str(),
        token,
    )
    .await?;

    let grades: HashMap<String, Vec<Option<f32>>> = parse_grades_html(html)?;
    Ok(serde_json::to_value(grades)?)
}

#[derive(Debug)]
struct Forms {
    form_build_id: String,
    form_token: String,
}

// this function gets the form token and form id
async fn fetch_export_form_tokens(token: String) -> Result<Forms, Box<dyn std::error::Error>> {
    let mut form_build_id = "N/A".to_string();
    let mut form_token = "N/A".to_string();

    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Cookie", token.parse().unwrap());

    let req = client
        .request(
            reqwest::Method::GET,
            "https://essexnorthshore.schoology.com/grades/export",
        )
        .headers(headers);

    let response = req.send().await?;
    let body = response.text().await?;

    let reV1 = regex::Regex::new(r#"name="form_build_id" id="([^"]+)""#).unwrap();
    if let Some(caps) = reV1.captures(body.as_str()) {
        form_build_id = caps[1].to_string();
        debug!("fetch_export_form_tokens: form token match found");
    } else {
        debug!("fetch_export_form_tokens: form token NO match found");
    }

    let reV2 =
        regex::Regex::new(r#"<input type="hidden" name="form_token" id="edit-s-grades-export-form-form-token-1" value="([^"]+)""#).unwrap();
    if let Some(caps) = reV2.captures(body.as_str()) {
        form_token = caps[1].to_string();
        debug!("fetch_export_form_tokens: form token match found");
    } else {
        debug!("fetch_export_form_tokens: form token NO match found");
    }

    let output = Forms {
        form_build_id: form_build_id,
        form_token: form_token,
    };

    debug!("fetch_export_form_tokens output: {:?}", output);

    Ok(output)
}

// This inputs the form build and form id from the last function (fetch_export_form_tokens)
// and selects the grading period
async fn select_grade_period(token: String) -> Result<Forms, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert("Cookie", token.parse().unwrap());

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

    let mut params = std::collections::HashMap::new();
    params.insert("grading_period[1113181]", "1113181"); // ! CHANGE THIS FOR THE GRADING PERIOD //
    // Q1
    params.insert("grading_period[1113182]", "1113182"); // ! CHANGE THIS FOR THE GRADING PERIOD 
    // Q2
    params.insert("grading_period[1113183]", "1113183"); // ! CHANGE THIS FOR THE GRADING PERIOD
    // Q3
    params.insert("grading_period[1113184]", "1113184"); // ! CHANGE THIS FOR THE GRADING PERIOD 
    // Q4
    params.insert("form_id", "s_grades_export_form");
    params.insert("op", "Next");
    let params_needed = fetch_export_form_tokens(token).await.unwrap();
    params.insert("form_build_id", &params_needed.form_build_id.as_str());
    params.insert("form_token", &params_needed.form_token.as_str());

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

    let reV1 = regex::Regex::new(r#"name="form_build_id" id="([^"]+)""#).unwrap();
    if let Some(caps) = reV1.captures(body.as_str()) {
        form_build_id = caps[1].to_string();
        debug!("select_grade_period: form token match found");
    } else {
        debug!("select_grade_period: form token NO match found");
    }

    let reV2 = regex::Regex::new(r#"form-token" value="([^"]+)""#).unwrap();
    if let Some(caps) = reV2.captures(body.as_str()) {
        form_token = caps[1].to_string();
        debug!("select_grade_period: form token match found");
    } else {
        debug!("select_grade_period: form token NO match found");
    }

    let output = Forms {
        form_build_id: form_build_id,
        form_token: form_token,
    };

    debug!("select_grade_period output: {:?}", output);

    Ok(output)
}

async fn fetch_class_ids(
    token: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Cookie", token.parse().unwrap());

    let req = client
        .request(
            reqwest::Method::GET,
            "https://essexnorthshore.schoology.com/grades/grades",
        )
        .headers(headers);

    let response = req.send().await?;
    let body = response.text().await?;

    let mut hashmap: HashMap<String, String> = HashMap::new();

    let re_class_id = regex::Regex::new(r#"id="s-js-gradebook-course-(\d+)"#).unwrap();
    for (_, [id]) in re_class_id.captures_iter(&body).map(|c| c.extract()) {
        trace!("{id}");
        hashmap.insert(format!("courses[{}][selected]", id), "1".to_string());
    }
    Ok(hashmap)
}

// Selects classes and gets the final export, creates html file
async fn fetch_final_grades_export(
    form_build_id: &str,
    form_token: &str,
    token: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder().build()?;

    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert("Cookie", token.parse().unwrap());

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

    let mut params = fetch_class_ids(&token).await.unwrap();
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

fn parse_grades_html(
    html: String,
) -> Result<HashMap<String, Vec<Option<f32>>>, Box<dyn std::error::Error>> {
    let document = scraper::Html::parse_document(html.as_str());
    let grade_selector =
        scraper::Selector::parse("td.grade, td.grade.final-grade, td.grade.no-grade").unwrap();
    let row_selector = scraper::Selector::parse("tr").unwrap();

    let mut course_grades: HashMap<String, Vec<Option<f32>>> = HashMap::new();
    let mut current_course: Option<String> = None;
    let mut grade_count = 0;

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
            grade_count = 0;
        } else if let Some(course) = &current_course {
            if grade_count >= 4 {
                continue;
            }

            for grade_cell in row.select(&grade_selector) {
                if grade_count >= 4 {
                    continue;
                }
                let grade_text = grade_cell
                    .text()
                    .collect::<String>()
                    .trim()
                    .replace("%", "");
                let grade = grade_text.parse::<f32>().ok();
                course_grades.get_mut(course).unwrap().push(grade);
                grade_count += 1;
            }
        }
    }

    Ok(course_grades)
}
