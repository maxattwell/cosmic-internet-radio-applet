{
  lib,
  rustPlatform,
  pkg-config,
  just,
  stdenv,
  libcosmicAppHook,
  gstreamer,
  gst-plugins-base,
  gst-plugins-good,
  gst-plugins-bad,
  gst-plugins-ugly,
  wayland,
  libxkbcommon,
  libGL,
  makeWrapper,
}:

rustPlatform.buildRustPackage rec {
  pname = "cosmic-internet-radio-applet";
  version = "0.1.0";

  src = lib.cleanSource ../.;

  cargoHash = "sha256-dKjsfn7l4i54GUKus25vxR2oDzbMjaIKTFmfCNq0lZM=";

  nativeBuildInputs = [
    pkg-config
    just
    libcosmicAppHook
    makeWrapper
  ];

  buildInputs = [
    gstreamer
    gst-plugins-base
    gst-plugins-good
    gst-plugins-bad
    gst-plugins-ugly
    wayland
    libxkbcommon
    libGL
  ];

  # GStreamer needs to find its plugins
  postInstall = ''
    wrapProgram $out/bin/cosmic-internet-radio-applet \
      --prefix GST_PLUGIN_SYSTEM_PATH_1_0 : "$GST_PLUGIN_SYSTEM_PATH_1_0"
  '';

  dontUseJustBuild = true;
  dontUseJustCheck = true;

  justFlags = [
    "--set"
    "prefix"
    (placeholder "out")
    "--set"
    "cargo-target-dir"
    "target/${stdenv.hostPlatform.rust.cargoShortTarget}"
  ];

  meta = with lib; {
    description = "An internet radio applet for the COSMIC desktop";
    homepage = "https://github.com/maxattwell/cosmic-internet-radio-applet";
    license = licenses.mpl20;
    maintainers = [ ];
    platforms = platforms.linux;
    mainProgram = "cosmic-internet-radio-applet";
  };
}
