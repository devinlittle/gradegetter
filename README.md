# Grade Getter

**ts pulls your grades straight from Schoology, no clicking around, no (offical) API access required .**

I hate schoology sm its so slow and the UI is soo outdated, slow, and their servers are stright ass.

---

#### What This Does

* Exposes your schoology grades in an API and clean JSON

* Refreshes token so its always alive and well

* refreshes grades every 15 seconds so they are up to date
  
  __DEEP DOWN IT...__

* Pulls grading period tokens

* Selects the grading quarter (Q1–Q4)

* Grabs your classes + final grades

* Parses the grades into a HashMap

* Saves the raw HTML in index.html just in case

## SETUP

Ensure the `tokengetter/config.json` has

```json
{
        "email": "x.y@hawks.tech", obviously ur email and password and not this
        "password": "password123",
        "browser": "/Applications/Chromium.app/Contents/MacOS/Chromium" -- PATH TO CHROMIUM BINARY
}
```

```bash
cd tokengetter
pnpm i
pnpm approve-builds # note this part is interactive 
echo "DONE!!!"
```

## RUNNING IT

```bash
cargo build --release
./target/release/gradegetter
```

# 

#### Function Rundown

| Function                      | funtion description                                                       |
| ----------------------------- | ------------------------------------------------------------------------- |
| `fetch_export_form_tokens()`  | Grabs the form tokens/build_id Schoology finna gonna hide                 |
| `select_grade_period()`       | Picks the grading quarter (change the IDs)                                |
| `fetch_final_grades_export()` | selects your classes and after pulls that grade HTML after                |
| `parse_grades_html()`         | Parses the mess of HTML into a usable `HashMap<String, Vec<Option<f32>>>` |

### OUTPUT (ignore my bad social studdies grade i was looking at the damn rust book in class)

```bash
[devin@gentoo-vm gradegetter]$ curl http://0.0.0.0:3000/grades
{
  "Freshmen Seminar": [
    89.0,
    93.2,
    93.0,
    76.75
  ],
  "Spanish I CP": [
    93.15,
    80.19,
    94.86,
    91.0
  ],
  "Geometry Honors": [
    95.08,
    89.56,
    90.24,
    80.1
  ],
  "World History Honors": [
    75.8,
    62.13,
    82.82,
    68.75
  ],
  "Biology I Honors": [
    87.41,
    83.96,
    93.13,
    96.01
  ],
  "English 9 Honors": [
    88.0,
    77.0,
    88.0,
    81.0
  ]
}
```

### NOTES

1. I need to add a way to make the class and quarters more user friendly. But this is kinda linux only and requires a bunch of knowledge....i just need to document how this works more
2. Any issues? hmu on the issues tab
