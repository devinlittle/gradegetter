# GradeGetter

### Setup:

```bash
cp ./backend/env.template ./.env
# To generate ENCRYPTION_KEY...
echo $(openssl rand -base64 32) >> .env
# Adjust ENV VARS to 
cargo build --release --bin gradegetter
```

#### Function Rundown

| Function                      | funtion description                                                       |
| ----------------------------- |:-------------------------------------------------------------------------:|
| `fetch_export_form_tokens()`  | Grabs the form tokens/build_id Schoology finna gonna hide                 |
| `select_grade_period()`       | Picks the grading quarter (change the IDs)                                |
| `fetch_final_grades_export()` | selects your classes and after pulls that grade HTML after                |
| `parse_grades_html()`         | Parses the mess of HTML into a usable `HashMap<String, Vec<Option<f32>>>` |
