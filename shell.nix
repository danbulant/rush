{ pkgs ? import <nixpkgs> {} }:
let
    unstable = import (fetchTarball "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz") { };
    # rust-rover things
    rust-toolchain =
        pkgs.symlinkJoin {
            name = "rust-toolchain";
            paths = with unstable; [rustc cargo rustPlatform.rustcSrc clippy rustfmt gcc rust-analyzer];
        };
in
pkgs.mkShell rec {
    buildInputs = with unstable;[
        openssl
        pkg-config
        cmake
        zlib
        rust-toolchain

        cargo-flamegraph
        hyperfine
        valgrind

        lld
    ];
    nativeBuildInputs = with unstable; [
        pkg-config
    ];
    LD_LIBRARY_PATH = "${unstable.lib.makeLibraryPath buildInputs}";
    OPENSSL_DIR="${unstable.openssl.dev}";
    OPENSSL_LIB_DIR="${unstable.openssl.out}/lib";
    RUST_SRC_PATH = "${unstable.rust.packages.stable.rustPlatform.rustLibSrc}";
}
