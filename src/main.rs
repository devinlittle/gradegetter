use axum::{Router, routing::get};
use std::{collections::HashMap, str, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    process::Command,
    sync::RwLock,
};

#[tokio::main]
async fn main() {
    let token_rw = Arc::new(RwLock::new(String::new()));
    let grades_rw = Arc::new(RwLock::new(String::new()));

    // Token Getter Thread?
    let token_rw_task1 = Arc::clone(&token_rw);
    tokio::spawn(async move {
        loop {
            let token = String::from_utf8_lossy(
                &Command::new("node")
                    .arg("tokengetter/")
                    .output()
                    .await
                    .expect("failed")
                    .stdout,
            )
            .trim()
            .to_string();
            let mut token_write = token_rw_task1.write().await;
            *token_write = token;
            drop(token_write);
            tokio::time::sleep(std::time::Duration::from_secs(1800)).await // 30 minutes
        }
    });

    // Grade fetcher
    let token_rw_task2 = Arc::clone(&token_rw);
    let grades_rw_task2 = Arc::clone(&grades_rw);
    tokio::spawn(async move {
        loop {
            let token_read = token_rw_task2.read().await;

            let class_pick_vars = select_grade_period(token_read.to_string()).await.unwrap();
            let html = fetch_final_grades_export(
                class_pick_vars.form_build_id.as_str(),
                class_pick_vars.form_token.as_str(),
                token_read.to_string(),
            )
            .await
            .unwrap();

            drop(token_read);

            let grades = match parse_grades_html(html) {
                Ok(data) => {
                    serde_json::to_string_pretty(&data).expect("failed to serialize grades")
                }
                Err(e) => {
                    eprintln!("Error parsing grades: {}", e);
                    format!("Error: {}", e)
                }
            };

            let mut grades_write = grades_rw_task2.write().await;
            *grades_write = grades;
            drop(grades_write);
            tokio::time::sleep(std::time::Duration::from_secs(15)).await
        }
    });

    let app = Router::new().route(
        "/grades",
        get({
            let grades = Arc::clone(&grades_rw);
            move || {
                let grades = Arc::clone(&grades);
                async move { grades_route(grades).await }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn grades_route(grades_rw: Arc<RwLock<String>>) -> (axum::http::StatusCode, String) {
    let grades_read = grades_rw.read().await;
    (axum::http::StatusCode::OK, grades_read.to_string())
}

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
    //        println!("Good 1x");
    } else {
        //        println!("No match found.");
    }

    let reV2 =
        regex::Regex::new(r#"<input type="hidden" name="form_token" id="edit-s-grades-export-form-form-token-1" value="([^"]+)""#).unwrap();
    if let Some(caps) = reV2.captures(body.as_str()) {
        form_token = caps[1].to_string();
    //        println!("Good 2x");
    } else {
        //        println!("No match found.");
    }

    let output = Forms {
        form_build_id: form_build_id,
        form_token: form_token,
    };

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
    //        println!("Good 3x");
    } else {
        //        println!("No match found. Class Pick V1");
    }

    let reV2 = regex::Regex::new(r#"form-token" value="([^"]+)""#).unwrap();
    if let Some(caps) = reV2.captures(body.as_str()) {
        form_token = caps[1].to_string();
    //        println!("Good 4x");
    } else {
        //        println!("No match found. Class Pick V2");
    }

    let output = Forms {
        form_build_id: form_build_id,
        form_token: form_token,
    };

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
    let body = response.text().await?;

    //    println!("{}", body);

    let file = File::create("index.html").await?;
    let mut writer = BufWriter::new(file);

    writer.write_all(body.as_bytes()).await?;

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
