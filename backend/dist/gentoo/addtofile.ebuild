BDEPEND="dev-libs/openssl"
#BDEPEND="virtual/pandoc"
SRC_URI+="http://voidsec.local:3000/devin/monorepo/archive/main.zip"

src_prepare() {
  default

  echo "$(pwd)"
  cp -r ../monorepo/* . || die
  cd apps/backend
}

src_configure() {
  echo $(pwd)
  cd apps/backend
  cargo_src_configure --frozen
}

src_install() {
  cargo_src_install

  #  pandoc --standalone -f markdown -t man ./res/amazingpacking.1.md >amazingpacking.1
  #  doman ./amazingpacking.1
}
