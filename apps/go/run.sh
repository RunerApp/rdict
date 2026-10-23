#!/bin/bash
# Build/run the MyGo replica. Requires Go 1.24+. The reader is pure Go,
# so no cgo or prebuilt static libs are needed.
set -euo pipefail
cd "$(dirname "$0")"

go run .
