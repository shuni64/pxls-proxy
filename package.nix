{
  lib,
  rustPlatform,
  cmake,
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
  ];

  meta = {
    description = "Reverse proxy for alternative Pxls frontends";
    homepage = "https://github.com/shuni64/pxls-proxy";
    license = lib.licenses.unlicense;
    maintainers = [ ];
  };
})
