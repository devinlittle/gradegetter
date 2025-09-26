# The backend!

## Setup

```bash
cp ./backend/env.template ./.env
# To generate ENCRYPTION_KEY...
echo $(openssl rand -base64 32) >> .env
# Adjust ENV VARS to 
cargo build --release --bin gradegetter
```

# Routes:

| Route                         | Input                                                                                                                                                                             | Function                             |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| `/auth/register`              | {<br/>    "username": "devin",<br/>    "password": "password"<br/>}                                                                                                               | Adds user to database                |
| `/auth/login`                 | {<br/> "username": "devin",<br/> "password": "password"<br/>}                                                                                                                     | returns JWT if login info is correct |
| `/auth/schoology/credentials` | {<br/>    "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.XXXXXX.XXXXXX",<br/>    "schoology_email": "first.last@hawks.tech",<br/>    "schoology_password": "PasswordofUser"<br/>} | adds schoology info to database      |
| `/grades`                     | {<br/>    "Authorization": "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.XXXXXX.XXXXXX"<br/>}                                                                                                     | returns grades                       |

