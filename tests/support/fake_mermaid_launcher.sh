#!/bin/sh
set -eu

exec /bin/sh "$(dirname "$0")/fake mmdc-body.sh" "$@"
