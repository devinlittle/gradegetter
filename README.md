# Grade Getter

A Rust powered monorepo to scrape Schoology grades, encrypt important information, and serve clean JSON. 

---

## Features

- **Real time Grade Scraping**: Fetches Schoology grades every 10 seconds.

- **AES Encryption**: Secures session tokens, emails, and passwords in PostgreSQL.

- **Multi user Support**: Scales from 10 to 100 users. 

#### Architecture:

| Program      | Function                                                                               |
| ------------ | -------------------------------------------------------------------------------------- |
| backend      | the backend service which interacts with the database safely                           |
| gradegetter  | fetches grades and formats them into valid json, putting them into a postgres database |
| gradegetter-desktop | views grades from api on a desktop native app                                   |
| tokengetter  | grabs schoology session token using puppeteer                                          |
| crypto_utils | the encryption and decryption crate                                                    |

#### Tech Stack:

* [Rust](https://rust-lang.org)

* [bun](https://bun.sh)/[pnpm](https://pnpm.io)

* [NodeJS](https://nodejs.org))

* [PostgreSQL](https://www.postgresql.org/)

#### What This Does:

* Exposes your schoology grades in an API and clean JSON

* Refreshes token so its always alive and well (every 30 minutes token will be refreshed)

* refreshes grades every 10 seconds so they are up to date
  
  __DEEP DOWN IT...__

* Pulls grading period tokens

* Selects the grading quarter (Q1–Q4)

* Grabs your classes + final grades

* Parses the grades into a HashMap 

#### Usage:

```bash
[devin@gentoo-vm gradegetter] curl --location 'http://0.0.0.0:3000/grades' \
--header 'Content-Type: application/json' \
--data '{
    "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJlM2E4YTAxZC0zYWRlLTQ5M2MtODE5OS00YmUxNjAzNTdiMzUiLCJ1c2VybmFtZSI6ImRldmluIiwiaWF0IjoxNzU4MTUyNDIzLCJleHAiOjE3ODk2ODg0MjN9.kYW2BeFDV0G_Wu1DjTS1l41QsnmlA3Xez8yIuicVcK0"
}'
{
    "Algebra II Honors": [
        null,
        null,
        null,
        null
    ],
    "Biology II Honors": [
        92.0500030517578,
        null,
        null,
        null
    ],
    "Computer Science Theory": [
        100.0,
        null,
        null,
        null
    ],
    "English 10 Honors": [
        100.0,
        null,
        null,
        null
    ],
    "Health Education 10": [
        92.5,
        null,
        92.5
    ],
    "Networking Essentials 10": [
        99.37999725341795,
        null,
        null,
        null
    ],
    "Spanish II CP": [
        null,
        null,
        null,
        null
    ],
    "U.S. Government and Politics CP": [
        100.0,
        null,
        100.0
    ],
    "United States History I CP": [
        100.0,
        null,
        null,
        null
    ]
}
```

