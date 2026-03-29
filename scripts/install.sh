#!/bin/sh
# Install the tracked mdv binary from the main branch.
# Usage:
#   curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/posaune0423/mdv/main/scripts/install.sh | sh
# Optional env:
#   MDV_INSTALL_DIR  directory for the binary (default: $HOME/.local/bin)

set -eu

REPO="posaune0423/mdv"
DEFAULT_INSTALL_DIR="${MDV_INSTALL_DIR:-$HOME/.local/bin}"
BINARY_URL="https://raw.githubusercontent.com/${REPO}/main/bin/mdv"
LOG_FILE=""
TTY_EFFECTS=0

if [ -t 1 ] && [ "${TERM:-}" != "dumb" ]; then
  TTY_EFFECTS=1
  RESET="$(printf '\033[0m')"
  BOLD="$(printf '\033[1m')"
  DIM="$(printf '\033[2m')"
  CYAN="$(printf '\033[36m')"
  GREEN="$(printf '\033[32m')"
  RED="$(printf '\033[31m')"
else
  RESET=""
  BOLD=""
  DIM=""
  CYAN=""
  GREEN=""
  RED=""
fi

die() {
  printf '%smdv install:%s %s\n' "$RED" "$RESET" "$1" >&2
  if [ -n "$LOG_FILE" ] && [ -f "$LOG_FILE" ]; then
    printf '\n%sLast log lines:%s\n' "$DIM" "$RESET" >&2
    tail -n 40 "$LOG_FILE" >&2 || true
  fi
  exit 1
}

spinner() {
  pid="$1"
  label="$2"
  frame_index=0

  if [ "$TTY_EFFECTS" -ne 1 ]; then
    return 0
  fi

  while kill -0 "$pid" 2>/dev/null; do
    case "$frame_index" in
      0) frame="|" ;;
      1) frame="/" ;;
      2) frame="-" ;;
      *) frame="\\" ;;
    esac
    printf '\r\033[2K%s[%s]%s %s%s%s' "$CYAN" "$frame" "$RESET" "$BOLD" "$label" "$RESET"
    frame_index=$(( (frame_index + 1) % 4 ))
    sleep 0.1
  done
}

run_step() {
  label="$1"
  shift

  (
    "$@"
  ) >>"$LOG_FILE" 2>&1 &
  pid="$!"

  spinner "$pid" "$label"
  if ! wait "$pid"; then
    if [ "$TTY_EFFECTS" -eq 1 ]; then
      printf '\r\033[2K%s[!!]%s %s\n' "$RED" "$RESET" "$label" >&2
    else
      printf '%s[!!]%s %s\n' "$RED" "$RESET" "$label" >&2
    fi
    die "$label failed"
  fi

  if [ "$TTY_EFFECTS" -eq 1 ]; then
    printf '\r\033[2K%s[ok]%s %s\n' "$GREEN" "$RESET" "$label"
  else
    printf '%s[ok]%s %s\n' "$GREEN" "$RESET" "$label"
  fi
}

print_banner() {
  printf '%s' "$GREEN"
  cat <<'EOF'
          _____                    _____                    _____          
         /\    \                  /\    \                  /\    \         
        /::\____\                /::\    \                /::\____\        
       /::::|   |               /::::\    \              /:::/    /        
      /:::::|   |              /::::::\    \            /:::/    /         
     /::::::|   |             /:::/\:::\    \          /:::/    /          
    /:::/|::|   |            /:::/  \:::\    \        /:::/____/           
   /:::/ |::|   |           /:::/    \:::\    \       |::|    |            
  /:::/  |::|___|______    /:::/    / \:::\    \      |::|    |     _____  
 /:::/   |::::::::\    \  /:::/    /   \:::\ ___\     |::|    |    /\    \ 
/:::/    |:::::::::\____\/:::/____/     \:::|    |    |::|    |   /::\____\
\::/    / ~~~~~/:::/    /\:::\    \     /:::|____|    |::|    |  /:::/    /
 \/____/      /:::/    /  \:::\    \   /:::/    /     |::|    | /:::/    / 
             /:::/    /    \:::\    \ /:::/    /      |::|____|/:::/    /  
            /:::/    /      \:::\    /:::/    /       |:::::::::::/    /   
           /:::/    /        \:::\  /:::/    /        \::::::::::/____/    
          /:::/    /          \:::\/:::/    /          ~~~~~~~~~~          
         /:::/    /            \::::::/    /                               
        /:::/    /              \::::/    /                                
        \::/    /                \::/____/                                 
         \/____/                  ~~                                       
                                                                           
EOF
  printf '%s' "$RESET"
}

command -v curl >/dev/null 2>&1 || die "curl is required"

install_dir="$DEFAULT_INSTALL_DIR"
case "$install_dir" in
/*) ;;
*) die "MDV_INSTALL_DIR must be an absolute path" ;;
esac

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT INT TERM
LOG_FILE="$tmp_dir/install.log"
staged_binary="$tmp_dir/mdv"

printf '%smdv installer%s\n' "$BOLD" "$RESET"
printf '%sInstalling the tracked main-branch binary into %s.%s\n' \
  "$DIM" "$install_dir" "$RESET"

run_step "Downloading main/bin/mdv" \
  curl -fL --proto '=https' --tlsv1.2 --retry 3 --retry-delay 1 -o "$staged_binary" "$BINARY_URL"

[ -f "$staged_binary" ] || die "download succeeded but no binary was written"

mkdir -p "$install_dir"
cp "$staged_binary" "$install_dir/mdv"
chmod +x "$install_dir/mdv"

printf '\n'
print_banner
printf '\n%sInstalled mdv to%s %s/mdv\n' "$BOLD" "$RESET" "$install_dir"

case ":${PATH:-}:" in
*:"$install_dir":*)
  printf '%sPATH already includes %s.%s\n' "$GREEN" "$install_dir" "$RESET"
  ;;
*)
  printf '%sAdd this to your shell profile if needed:%s export PATH="%s:$PATH"\n' \
    "$DIM" "$RESET" "$install_dir"
  ;;
esac
