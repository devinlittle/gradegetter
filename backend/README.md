# The backend!

## Has support for the following distros:

* Alpine
* Arch
* Debian (need cargo-deb installed)
* Gentoo (need pycargoebuild installed)
* Void

generate secret encryp[tion kety
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

    
