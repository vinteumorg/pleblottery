{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.bitcoind
  ];

  shellHook = ''
    ./regtest.sh
  '';
}

