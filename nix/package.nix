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
  gst-libav,
  wayland,
  libxkbcommon,
  libGL,
  makeWrapper,
  glib-networking,
}:

rustPlatform.buildRustPackage rec {
  pname = "cosmic-ext-applet-radio";
  version = "0.1.0";

  src = lib.cleanSource ../.;

  cargoHash = "sha256-8p7aLdyjboD7W0OIFp9ssQHEH0IfVT2jFf9WipZ686E=";

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
    gst-libav
    wayland
    libxkbcommon
    libGL
    glib-networking
  ];

  # GStreamer needs to find its plugins and TLS support
  postInstall = ''
    wrapProgram $out/bin/cosmic-ext-applet-radio \
      --prefix GST_PLUGIN_SYSTEM_PATH_1_0 : "$GST_PLUGIN_SYSTEM_PATH_1_0" \
      --prefix GIO_EXTRA_MODULES : "${glib-networking}/lib/gio/modules"
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
    homepage = "https://github.com/maxattwell/cosmic-ext-applet-radio";
    license = licenses.mpl20;
    maintainers = [ ];
    platforms = platforms.linux;
    mainProgram = "cosmic-ext-applet-radio";
  };
}
