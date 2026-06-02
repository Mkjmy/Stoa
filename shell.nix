{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    rustc
    pkg-config
    jetbrains-mono
    
    # GUI Libs
    libX11
    libXcursor
    libXrandr
    libXi
    libGL
    
    # Wayland support (optional but good to have)
    wayland
    libxkbcommon
    
    # Audio (if needed)
    alsa-lib
    
    # Vulnerability/GPU
    vulkan-loader
  ];

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    libX11
    libXcursor
    libXrandr
    libXi
    libGL
    wayland
    libxkbcommon
    vulkan-loader
  ]);
}
