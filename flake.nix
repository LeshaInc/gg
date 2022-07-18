{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let 
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShell = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            pkgconfig
            llvmPackages.bintools
          ];

          buildInputs = with pkgs; [
            vulkan-loader vulkan-validation-layers
            xlibsWrapper xorg.libXcursor xorg.libXrandr xorg.libXi
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d/";
          RUST_LOG = "warn";
          RUST_BACKTRACE = 1;
        };
      }
    );
}
