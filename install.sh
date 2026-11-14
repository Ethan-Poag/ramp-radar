#!/bin/sh
# Ramp Radar Linux installer.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Ethan-Poag/ramp-radar/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/Ethan-Poag/ramp-radar/main/install.sh | sh -s -- --server
# Safer: download this file, read it, then run  sh install.sh  (or  sh install.sh --server).
set -eu

REPO='Ethan-Poag/ramp-radar'
BASE="${RR_DOWNLOAD_BASE:-https://github.com/${REPO}/releases/latest/download}"
server=0
dry_run=0
prefix=''

usage() {
  cat <<'EOF'
Install Ramp Radar on Linux.

Usage: sh install.sh [options]

  (default)   Desktop app: .deb via apt on Debian/Ubuntu, .rpm via dnf/yum
              on Fedora/RHEL, otherwise an AppImage into ~/.local/bin.
  --server    Headless server for this machine's architecture, plus a
              systemd service (browser UI at http://<host>:8080).
  --prefix DIR
              Install under DIR instead of system paths (no systemd,
              useful for testing).
  --dry-run   Print what would be installed, without downloading or writing.
  -h, --help  Show this help.

Environment:
  RR_DOWNLOAD_BASE   Override the release download URL (default: GitHub latest).
  RR_ACCESS_TOKEN    If set, used as the server access token; otherwise one is
                     generated and printed when installing the service.
EOF
}

while [ $# -gt 0 ]; do
  case $1 in
    --server) server=1 ;;
    --dry-run) dry_run=1 ;;
    --prefix)
      if [ $# -lt 2 ]; then
        echo "install.sh: --prefix needs a directory" >&2
        exit 2
      fi
      prefix=$2
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "install.sh: unknown option: $1" >&2
      echo "Try  sh install.sh --help" >&2
      exit 2
      ;;
  esac
  shift
done

die() {
  echo "install.sh: $*" >&2
  exit 1
}

info() {
  echo "$*"
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "need '$1' on PATH, but it was not found"
  fi
}

need_sudo() {
  if [ "$(id -u)" -eq 0 ]; then
    echo ''
    return
  fi
  if command -v sudo >/dev/null 2>&1; then
    echo sudo
    return
  fi
  die "this step needs root. Install sudo, or re-run as root."
}

download() {
  url=$1
  dest=$2
  info "Downloading $url"
  if [ "$dry_run" -eq 1 ]; then
    info "[dry run] would save to $dest"
    return 0
  fi
  curl -fL --retry 3 --retry-delay 2 -o "$dest" "$url" \
    || die "download failed: $url"
}

arch=$(uname -m)
case $arch in
  x86_64|amd64) arch=x86_64 ;;
  aarch64|arm64) arch=aarch64 ;;
  *) die "unsupported architecture: $arch (need x86_64 or aarch64)" ;;
esac

sys=$(uname -s)
if [ "$sys" != Linux ]; then
  die "this script installs the Linux build. For Windows and macOS, use the download links in the README: https://github.com/${REPO}#download"
fi

ID=''
ID_LIKE=''
if [ -f /etc/os-release ]; then
  # shellcheck disable=SC1091
  . /etc/os-release
fi

is_debian() {
  case ${ID:-} in
    debian|ubuntu|linuxmint|pop|raspbian|elementary|neon) return 0 ;;
  esac
  case " ${ID_LIKE:-} " in
    *' debian '*|*' ubuntu '*) return 0 ;;
  esac
  return 1
}

is_fedora_family() {
  case ${ID:-} in
    fedora|rhel|centos|rocky|almalinux|ol|amzn) return 0 ;;
  esac
  case " ${ID_LIKE:-} " in
    *' fedora '*|*' rhel '*|*' centos '*) return 0 ;;
  esac
  return 1
}

# --- desktop --------------------------------------------------------------

install_desktop() {
  if [ "$arch" != x86_64 ]; then
    die "a desktop installer is not published for $arch Linux yet. On a Raspberry Pi or ARM machine, install the browser/server version instead:
  sh install.sh --server"
  fi

  if is_debian && command -v apt-get >/dev/null 2>&1; then
    info "Debian/Ubuntu desktop install ($arch)"
    tmp=$(mktemp -d)
    # shellcheck disable=SC2064
    trap 'rm -rf "$tmp"' EXIT INT HUP
    deb="$tmp/RampRadar-linux-amd64.deb"
    download "$BASE/RampRadar-linux-amd64.deb" "$deb"
    if [ "$dry_run" -eq 1 ]; then
      info "[dry run] sudo apt-get update && sudo apt-get install -y $deb"
    else
      su=$(need_sudo)
      # $su is unquoted so an empty value (already root) is ignored.
      # shellcheck disable=SC2086
      $su apt-get update
      # shellcheck disable=SC2086
      $su apt-get install -y "$deb"
    fi
    info "Done. Open Ramp Radar from your application menu."
    return
  fi

  if is_fedora_family && { command -v dnf >/dev/null 2>&1 || command -v yum >/dev/null 2>&1; }; then
    pkg=dnf
    command -v dnf >/dev/null 2>&1 || pkg=yum
    info "Fedora/RHEL desktop install ($arch) using $pkg"
    tmp=$(mktemp -d)
    # shellcheck disable=SC2064
    trap 'rm -rf "$tmp"' EXIT INT HUP
    rpm="$tmp/RampRadar-linux-x86_64.rpm"
    download "$BASE/RampRadar-linux-x86_64.rpm" "$rpm"
    if [ "$dry_run" -eq 1 ]; then
      info "[dry run] sudo $pkg install -y $rpm"
    else
      su=$(need_sudo)
      # shellcheck disable=SC2086
      $su "$pkg" install -y "$rpm"
    fi
    info "Done. Open Ramp Radar from your application menu."
    return
  fi

  info "Installing the AppImage into ~/.local/bin ($arch)"
  bindir=${prefix:-$HOME/.local/bin}
  appdir=${prefix:-$HOME/.local/share/applications}
  dest=$bindir/ramp-radar.AppImage
  if [ "$dry_run" -eq 1 ]; then
    info "[dry run] would download $BASE/RampRadar-linux-x86_64.AppImage to $dest"
    info "[dry run] would write $appdir/ramp-radar.desktop"
    return
  fi
  mkdir -p "$bindir" "$appdir"
  download "$BASE/RampRadar-linux-x86_64.AppImage" "$dest"
  chmod +x "$dest"
  if [ ! -e "$bindir/ramp-radar" ]; then
    ln -s "$dest" "$bindir/ramp-radar"
  fi
  cat > "$appdir/ramp-radar.desktop" <<EOF
[Desktop Entry]
Name=Ramp Radar
Comment=Get alerted when watched aircraft approach an airport
Exec=$dest
Terminal=false
Type=Application
Categories=Utility;
StartupNotify=true
EOF
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$appdir" 2>/dev/null || true
  fi
  info "Done. Open Ramp Radar from your application menu, or run: $dest"
  case :$PATH: in
    *":$bindir:"*) ;;
    *) info "Note: add $bindir to your PATH if the command is not found." ;;
  esac
}

# --- server ---------------------------------------------------------------

random_token() {
  dd if=/dev/urandom bs=16 count=1 2>/dev/null | od -An -tx1 | tr -d ' \n'
}

install_server() {
  if [ "$arch" = x86_64 ]; then
    tarball='ramp-radar-server-linux-x86_64.tar.gz'
  else
    tarball='ramp-radar-server-linux-arm64.tar.gz'
  fi

  if [ -n "$prefix" ]; then
    bindir=$prefix/bin
    unitdir=$prefix/etc/systemd/system
    datadir=$prefix/var/lib/ramp-radar
  else
    bindir=/usr/local/bin
    unitdir=/etc/systemd/system
    datadir=/var/lib/ramp-radar
  fi

  info "Server install ($arch) → $bindir/ramp-radar-server"
  tmp=$(mktemp -d)
  # shellcheck disable=SC2064
  trap 'rm -rf "$tmp"' EXIT INT HUP
  archive=$tmp/$tarball
  download "$BASE/$tarball" "$archive"

  if [ "$dry_run" -eq 1 ]; then
    info "[dry run] would extract $tarball and install the binary to $bindir"
    info "[dry run] would install ramp-radar.service to $unitdir"
    return 0
  fi

  tar -C "$tmp" -xzf "$archive"
  if [ ! -f "$tmp/ramp-radar-server" ]; then
    die "the archive did not contain ramp-radar-server"
  fi

  su=''
  if [ -z "$prefix" ]; then
    su=$(need_sudo)
  fi

  if [ -n "$su" ]; then
    $su mkdir -p "$bindir" "$unitdir"
    $su install -m755 "$tmp/ramp-radar-server" "$bindir/ramp-radar-server"
  else
    mkdir -p "$bindir" "$unitdir"
    install -m755 "$tmp/ramp-radar-server" "$bindir/ramp-radar-server"
  fi

  if [ -f "$tmp/ramp-radar.service" ]; then
    if [ -n "$su" ]; then
      $su install -m644 "$tmp/ramp-radar.service" "$unitdir/ramp-radar.service"
    else
      install -m644 "$tmp/ramp-radar.service" "$unitdir/ramp-radar.service"
    fi
  fi

  # Skip system user and systemd when staging under --prefix.
  if [ -z "$prefix" ]; then
    if ! getent passwd ramp-radar >/dev/null 2>&1; then
      $su useradd --system --create-home --home-dir "$datadir" ramp-radar \
        || die "could not create the ramp-radar system user"
    fi
    $su mkdir -p "$datadir"
    $su chown ramp-radar:ramp-radar "$datadir"
  else
    mkdir -p "$datadir"
  fi

  token=${RR_ACCESS_TOKEN:-}
  generated=0
  if [ -z "$token" ]; then
    token=$(random_token)
    generated=1
  fi
  dropin=$unitdir/ramp-radar.service.d
  if [ -n "$su" ]; then
    $su mkdir -p "$dropin"
    printf '[Service]\nEnvironment=RR_ACCESS_TOKEN=%s\n' "$token" \
      | $su tee "$dropin/override.conf" >/dev/null
  else
    mkdir -p "$dropin"
    printf '[Service]\nEnvironment=RR_ACCESS_TOKEN=%s\n' "$token" \
      > "$dropin/override.conf"
  fi

  host=$(hostname 2>/dev/null || echo localhost)
  url="http://${host}:8080"
  started=0
  if [ -z "$prefix" ] && command -v systemctl >/dev/null 2>&1 && [ -d /run/systemd/system ]; then
    $su systemctl daemon-reload
    $su systemctl enable --now ramp-radar
    started=1
    info "Service enabled and started."
  else
    info "systemd was not started (prefix install, or no systemd on this machine)."
    info "When you are ready:  sudo systemctl enable --now ramp-radar"
  fi

  info "Open $url"
  info "First visit: ${url}/?token=${token}"
  if [ "$generated" -eq 1 ]; then
    info "Access token (save this; it will not be shown again): $token"
  else
    info "Using the access token from RR_ACCESS_TOKEN."
  fi
  if [ "$started" -eq 1 ]; then
    info "Done. The front desk can bookmark $url"
  else
    info "Binary installed at $bindir/ramp-radar-server"
  fi
}

need_cmd curl
need_cmd tar
need_cmd install
need_cmd mktemp

if [ "$server" -eq 1 ]; then
  install_server
else
  install_desktop
fi
