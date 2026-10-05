#!/bin/bash
# Uploads the DARs one asset needs to a Canton participant through the
# participant's admin API: first the shared dependencies, then the asset's
# own DARs.
#
# Usage: dars/upload_dars.sh <asset>
#   <asset> is cbtc or beth.
#
# Environment:
#   jwt_token             the participant admin token. Leave it unset when the
#                         admin API checks no token.
#   canton_admin_api_url  the admin API address, by default localhost:5002.
#
# The script needs grpcurl and jq. Canton skips a DAR that the participant
# already holds, so a second run is safe.
set -euo pipefail

asset="${1:-}"
case "${asset}" in
  cbtc | beth) ;;
  *)
    echo "Usage: $0 <asset>, where <asset> is one of: cbtc, beth" >&2
    exit 2
    ;;
esac

dar_root="$(cd "$(dirname "$0")" && pwd)"
jwt_token="${jwt_token:-}"
canton_admin_api_url="${canton_admin_api_url:-localhost:5002}"
package_service="com.digitalasset.canton.admin.participant.v30.PackageService"

# grpcurl expands ${jwt_token} itself, so the token stays out of the process
# list. The single quotes are deliberate.
auth=()
if [ -n "${jwt_token}" ]; then
  export jwt_token
  auth=(-expand-headers -H 'Authorization: Bearer ${jwt_token}')
fi

upload_dar() {
  local dar="$1"
  local name
  name="$(basename "${dar}")"
  echo "Uploading ${name}"
  base64 < "${dar}" | tr -d '\n' \
    | jq -Rc --arg name "${name}" \
      '{dars: [{bytes: ., description: $name}], vet_all_packages: true, synchronize_vetting: true}' \
    | grpcurl -plaintext ${auth[@]+"${auth[@]}"} -d @ \
      "${canton_admin_api_url}" "${package_service}.UploadDar"
}

upload_dir() {
  local dir="$1"
  local dars=("${dir}"/*.dar)
  if [ ! -e "${dars[0]}" ]; then
    echo "No DAR files in ${dir}" >&2
    exit 1
  fi
  for dar in "${dars[@]}"; do
    upload_dar "${dar}"
  done
}

echo "Uploading the dependency DARs..."
upload_dir "${dar_root}/dependencies"
echo "Uploading the ${asset} DARs..."
upload_dir "${dar_root}/${asset}"
echo "Uploaded every DAR that ${asset} needs."
