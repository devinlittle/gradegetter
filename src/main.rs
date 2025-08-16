use std::{collections::HashMap, env};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};

//const token: &str = "SESS948af67c60a38b4869db7f1955275d29=2a07cd84846d552d958aa8af012f32f3";

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let token: &str = &args[1];

    let class_pick_vars = class_pick(token.to_string()).await.unwrap();
    final_req(
        class_pick_vars.form_build_id.as_str(),
        class_pick_vars.form_token.as_str(),
        token.to_string(),
    )
    .await
    .unwrap();
    //    println!("D-Done! 🧖‍♀️");
}

struct initExportOutputs {
    form_build_id: String,
    form_token: String,
}

// this function gets the form token and form id
async fn init_export(token: String) -> Result<initExportOutputs, Box<dyn std::error::Error>> {
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
    /*    let output = format!(
        "form_build_id={0},form_token={1}",
        form_build_id, form_token
    )
    .to_string();
    Ok(output)*/

    let output = initExportOutputs {
        form_build_id: form_build_id,
        form_token: form_token,
    };

    Ok(output)
}

// This inputs the form build and form id from the last function (init_export)
// and selects the grading period
async fn class_pick(token: String) -> Result<initExportOutputs, Box<dyn std::error::Error>> {
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
    let paramsNeeded = init_export(token).await.unwrap();
    params.insert("grading_period[1070869]", "1070869"); // ! CHANGE THIS FOR THE GRADING PERIOD 
    params.insert("grading_period[1070866]", "1070866"); // ! CHANGE THIS FOR THE GRADING PERIOD 
    params.insert("grading_period[1070867]", "1070867"); // ! CHANGE THIS FOR THE GRADING PERIOD 
    params.insert("grading_period[1070868]", "1070868"); // ! CHANGE THIS FOR THE GRADING PERIOD 
    params.insert("form_id", "s_grades_export_form");
    params.insert("op", "Next");
    params.insert("form_build_id", &paramsNeeded.form_build_id.as_str());
    params.insert("form_token", &paramsNeeded.form_token.as_str());

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
    /*    let output = format!(
        "form_build_id={0},form_token={1}",
        form_build_id, form_token
    )
    .to_string();
    Ok(output)*/

    let output = initExportOutputs {
        form_build_id: form_build_id,
        form_token: form_token,
    };

    Ok(output)
}

// Selects classes and gets the final export, creates html file
async fn final_req(
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

    let mut params = std::collections::HashMap::new();
    params.insert("courses[7424299081][selected]", "1");
    params.insert("courses[7461240135][selected]", "1");
    params.insert("courses[7424298749][selected]", "1");
    params.insert("courses[7424300380][selected]", "1");
    params.insert("courses[7424298922][selected]", "1");
    params.insert("courses[7424299614][selected]", "1");
    params.insert("courses[7424299224][selected]", "1");
    params.insert("comment_gps[-1]", "-1");
    params.insert("op", "Submit");
    params.insert("form_id", "s_grades_export_form");
    params.insert("form_build_id", form_build_id);
    params.insert("form_token", form_token);

    let req = client
        .request(
            reqwest::Method::POST,
            "https://essexnorthshore.schoology.com/grades/export",
        )
        //    .headers(headers.clone())
        .headers(headers)
        .form(&params);
    let response = req.send().await?;
    let body = response.text().await?;

    //    println!("{}", body);

    let file = File::create("index.html").await?;
    let mut writer = BufWriter::new(file);

    writer.write_all(body.as_bytes()).await?;

    let document = scraper::Html::parse_document(body.as_str());
    let grade_selector =
        scraper::Selector::parse("td.grade, td.grade.final-grade, td.grade.no-grade").unwrap();
    let title_selector = scraper::Selector::parse("tr.course-title th h2").unwrap();
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

    println!("{}", serde_json::to_string_pretty(&course_grades).unwrap());

    /*    let req_2 = client
        .request(
            reqwest::Method::GET,
            "https://essexnorthshore.schoology.com/user/133626389/info",
        )
        .headers(headers.clone());

    let response = req_2.send().await?;
    let body = response.text().await?;

    let regex_req_v2 =
        regex::Regex::new(r#"<img[^>]*src="([^"]+)"[^>]*alt="[^"]*Profile picture for[^"]*""#)
            .unwrap();

    if let Some(caps) = regex_req_v2.captures(body.as_str()) {
        println!("{:?}", caps[1].to_string());
    } */

    Ok("Hi".to_string())
}
