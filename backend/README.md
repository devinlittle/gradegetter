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

 

# THIS FILE WILL BE FIXED LATER...

## Has support for the following distros:

* Alpine
* Arch
* Debian (need cargo-deb installed)
* Gentoo (need pycargoebuild installed)
* Void

generate secret encryption kety
`openssl rand -base64 32`

and technically windows + mac but WHO is gonna run server software on...WINDOWS😭😭

## Build the app

### Alpine: `command here` <-- alpie isnt my top priotity il work on it later but for now, command here

### Arch: `makepkg -si # running -si installs`

### Debian: `cargo deb --release && sudo dpkg -i ../../target/debian/backend/*.deb`<-- needs work

### Gentoo: `pycargoebuild . && cat dist/gentoo/addtofile.ebuild >> backend-*`

### Void: void doent have any good 1liners so il explain it below here

okay so basically in void you gotta clone the [void-packages](https://github.com/void-linux/void-packages) repo, 

In void-packages run `./xbps-src binary-bootstrap`

In the `apps/backend` dir run `cp -r dist/void/backend/ /path/to/void-packages/srcpkgs/`

And finally run `./xbps-src pkg backend`

to install `xbps-install -R hostdir/binpkgs backend` YAY!!!

    
