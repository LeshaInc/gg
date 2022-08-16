{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
  };

  outputs = { self, nixpkgs, utils, fenix }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        toolchain = with fenix.packages.${system};
          combine (with complete; [
            rustc
            rust-src
            cargo
            rustfmt
            rust-analyzer
          ]);
      in
      {
        devShell = with pkgs; mkShell rec {
          buildInputs = [
            toolchain
            rnix-lsp
            vulkan-loader
            vulkan-validation-layers
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d/";

          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
          RUST_LOG = "warn";
          RUST_BACKTRACE = 1;
        };
      }
    );
}
