{
  lib,
  rustPlatform,
  cmake,
  openssl,
  pkg-config,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "pxls-proxy";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    cmake # for libz-ng-sys
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  env.OPENSSL_NO_VENDOR = 1;

  meta = {
    description = "Reverse proxy for alternative Pxls frontends";
    homepage = "https://github.com/shuni64/pxls-proxy";
    license = lib.licenses.unlicense;
    maintainers = [ ];
  };
})
