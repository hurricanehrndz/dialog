{
  description = "Cross-platform swiftDialog-compatible dialog utility";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
            nodejs
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            # Tauri v2 Linux (dev host) dependencies
            gtk3
            webkitgtk_4_1
            libsoup_3
            openssl
            glib
            librsvg
          ];

          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.lib.optionalString pkgs.stdenv.isLinux
              "${pkgs.webkitgtk_4_1.dev}/lib/pkgconfig:${pkgs.libsoup_3.dev}/lib/pkgconfig"}:$PKG_CONFIG_PATH"
          '';
        };
      });
    };
}
