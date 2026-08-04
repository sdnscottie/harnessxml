#!/usr/bin/env bash
# Deploy harnessxml.com to GCP.
#
# Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
#
# Shape: GCS bucket + backend bucket + Cloud CDN, added as an ADDITIONAL HOST RULE
# on the EXISTING shared global HTTPS load balancer (IP 34.49.81.67) that already
# serves visml.com, rumima.agrarobotics.com and collab.*. Same pattern as
# visml.com. No VM, no Cloud Run, no container.
#
#   bash deploy/deploy_harnessxml.sh            build + deploy everything
#   bash deploy/deploy_harnessxml.sh --content  upload the site only (fast path)
#   bash deploy/deploy_harnessxml.sh --dry-run  print what would change
#
# Idempotent: safe to re-run.

set -euo pipefail

# ---- CONFIG -----------------------------------------------------------------
PROJECT_ID="agrarobotics-licensing"
DOMAIN="harnessxml.com"
WWW_DOMAIN="www.harnessxml.com"

BUCKET="harnessxml-web"
BUCKET_LOCATION="EU"
BACKEND="harnessxml-backend"
CERT="harnessxml-cert"
ARMOR_POLICY="harnessxml-edge-policy"
PATH_MATCHER="harnessxml-pm"

# The EXISTING shared load balancer. Do not create a second one.
LB_URLMAP="rumima-urlmap"
LB_HTTPS_PROXY="rumima-https-proxy"
LB_IP_NAME="rumima-lb-ip"
# -----------------------------------------------------------------------------

HERE="$(cd "$(dirname "$0")/.." && pwd)"
PUBLIC="$HERE/site/public"

CONTENT_ONLY=0
DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    --content) CONTENT_ONLY=1 ;;
    --dry-run) DRY_RUN=1 ;;
    *) echo "unknown flag: $arg" >&2; exit 2 ;;
  esac
done

run() {
  if [[ $DRY_RUN -eq 1 ]]; then
    echo "  DRY-RUN: $*"
  else
    "$@"
  fi
}

say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

say "Project"
run gcloud config set project "$PROJECT_ID"

# ---------------------------------------------------------------- build
say "Build the site"
python3 "$HERE/site/build.py" --check
test -f "$PUBLIC/index.html" || { echo "build produced no index.html"; exit 1; }

if [[ $CONTENT_ONLY -eq 0 ]]; then

  say "APIs"
  run gcloud services enable compute.googleapis.com storage.googleapis.com \
      certificatemanager.googleapis.com logging.googleapis.com monitoring.googleapis.com

  # ------------------------------------------------------------ bucket
  say "Bucket gs://$BUCKET"
  if ! gcloud storage buckets describe "gs://$BUCKET" >/dev/null 2>&1; then
    run gcloud storage buckets create "gs://$BUCKET" \
        --location="$BUCKET_LOCATION" --uniform-bucket-level-access
  else
    echo "   (exists)"
  fi

  # mainPageSuffix makes /spec/v1.0/foo/ resolve to .../foo/index.html, which is
  # what gives the specification clean, citable URLs with no .html in them.
  #
  # notFoundPage is 404.html and NOT index.html. visml-web uses index.html, so a
  # mistyped URL there returns the HOMEPAGE with HTTP 200 — fine for a one-page
  # site, wrong for a documentation site: it hides broken links from readers and
  # lets search engines index infinite duplicate homepages.
  say "Website config (clean URLs + a real 404)"
  run gcloud storage buckets update "gs://$BUCKET" \
      --web-main-page-suffix=index.html --web-error-page=404.html

  say "Public read"
  run gcloud storage buckets add-iam-policy-binding "gs://$BUCKET" \
      --member=allUsers --role=roles/storage.objectViewer

  # ------------------------------------------------------------ backend bucket
  say "Backend bucket + Cloud CDN"
  # Security headers. visml.com currently sends NONE of these and leaks
  # `server: UploadServer`; harnessxml.com sets them from day one.
  # No inline <script> without a hash would break the theme bootstrap, so
  # 'unsafe-inline' is scoped to script-src only for that one inline block.
  CSP="default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'"

  if ! gcloud compute backend-buckets describe "$BACKEND" >/dev/null 2>&1; then
    run gcloud compute backend-buckets create "$BACKEND" \
        --gcs-bucket-name="$BUCKET" \
        --enable-cdn \
        --cache-mode=CACHE_ALL_STATIC \
        --default-ttl=3600 \
        --client-ttl=3600 \
        --max-ttl=86400 \
        --compression-mode=AUTOMATIC \
        --custom-response-header="Strict-Transport-Security: max-age=31536000; includeSubDomains; preload" \
        --custom-response-header="X-Content-Type-Options: nosniff" \
        --custom-response-header="Referrer-Policy: strict-origin-when-cross-origin" \
        --custom-response-header="Permissions-Policy: geolocation=(), microphone=(), camera=(), interest-cohort=()" \
        --custom-response-header="Content-Security-Policy: $CSP" \
        --custom-response-header="Cross-Origin-Opener-Policy: same-origin" \
        --custom-response-header="X-Frame-Options: DENY"
  else
    echo "   (exists — updating CDN + headers)"
    run gcloud compute backend-buckets update "$BACKEND" \
        --enable-cdn --cache-mode=CACHE_ALL_STATIC \
        --default-ttl=3600 --client-ttl=3600 --max-ttl=86400 \
        --compression-mode=AUTOMATIC
  fi

  # ------------------------------------------------------------ Cloud Armor
  # NOTE: a backend BUCKET supports only EDGE security policies — IP/geo/header
  # allow-deny at the CDN edge. Full backend policies (OWASP preset ruleset, rate
  # limiting, bot management) require a backend SERVICE. Acceptable here: the
  # OWASP ruleset defends an APPLICATION, and this site is static, public,
  # read-only, with no forms, no auth and no server-side code. Volumetric DDoS is
  # absorbed by Google's front end and the CDN regardless.
  say "Cloud Armor edge policy"
  if ! gcloud compute security-policies describe "$ARMOR_POLICY" >/dev/null 2>&1; then
    run gcloud compute security-policies create "$ARMOR_POLICY" \
        --type=CLOUD_ARMOR_EDGE \
        --description="Edge policy for harnessxml.com (static docs site)"
  else
    echo "   (exists)"
  fi
  run gcloud compute backend-buckets update "$BACKEND" \
      --edge-security-policy="$ARMOR_POLICY"

  # ------------------------------------------------------------ certificate
  say "Managed SSL certificate"
  if ! gcloud compute ssl-certificates describe "$CERT" --global >/dev/null 2>&1; then
    run gcloud compute ssl-certificates create "$CERT" --global \
        --domains="$DOMAIN,$WWW_DOMAIN"
  else
    echo "   (exists)"
  fi

  # ⚠ --ssl-certificates REPLACES the whole list; it does not append. The proxy
  # currently carries rumima-cert-2, collab-cert, visml-cert and visml-apex-cert.
  # Passing only $CERT would instantly break TLS for rumima.agrarobotics.com,
  # collab.agrarobotics.com, collab.visml.com, visml.com and www.visml.com.
  # So: read the current list, append, write back.
  say "Attach the certificate WITHOUT dropping the existing four"
  EXISTING="$(gcloud compute target-https-proxies describe "$LB_HTTPS_PROXY" --global \
              --format='value(sslCertificates)' | tr ';,' '\n\n' | sed 's#.*/##' \
              | grep -v '^$' | sort -u | paste -sd, -)"
  echo "   existing certs: $EXISTING"
  if echo ",$EXISTING," | grep -q ",$CERT,"; then
    echo "   ($CERT already attached — leaving the proxy alone)"
  else
    run gcloud compute target-https-proxies update "$LB_HTTPS_PROXY" --global \
        --ssl-certificates="${EXISTING},${CERT}"
  fi

  # ------------------------------------------------------------ host rule
  say "Host rule on the EXISTING url-map ($LB_URLMAP)"
  if gcloud compute url-maps describe "$LB_URLMAP" --format='value(hostRules.hosts)' \
       | tr ';,' '\n\n' | grep -qx "$DOMAIN"; then
    echo "   (host rule for $DOMAIN already present)"
  else
    run gcloud compute url-maps add-path-matcher "$LB_URLMAP" \
        --path-matcher-name="$PATH_MATCHER" \
        --default-backend-bucket="$BACKEND" \
        --new-hosts="$DOMAIN,$WWW_DOMAIN"
  fi
  # HTTP->HTTPS is already covered: url-map `rumima-http-redirect` uses a
  # defaultUrlRedirect with httpsRedirect and enumerates no hosts, so a new host
  # is redirected automatically. Nothing to add.
fi

# ---------------------------------------------------------------- content
say "Upload site content"

# Cache strategy. A released specification version is IMMUTABLE (governance §4),
# so it may be cached for a year. Everything else is short, because the site is
# edited and a stale nav is worse than a cache miss.
run gcloud storage rsync -r -d "$PUBLIC" "gs://$BUCKET" \
    --cache-control="public, max-age=300"

run gcloud storage objects update "gs://$BUCKET/assets/**" \
    --cache-control="public, max-age=86400"

run gcloud storage objects update "gs://$BUCKET/schema/**" \
    --cache-control="public, max-age=31536000, immutable"

run gcloud storage objects update "gs://$BUCKET/examples-src/**" \
    --cache-control="public, max-age=3600"

say "Invalidate the CDN for the mutable paths"
run gcloud compute url-maps invalidate-cdn-cache "$LB_URLMAP" --path="/*" --async

# ---------------------------------------------------------------- done
LB_IP="$(gcloud compute addresses describe "$LB_IP_NAME" --global --format='value(address)' 2>/dev/null || echo '34.49.81.67')"

cat <<EOF

$(printf '\033[1mDone.\033[0m')

  DNS — add these at the registrar, then the managed cert provisions itself:

      ${DOMAIN}.        A    ${LB_IP}
      ${WWW_DOMAIN}.    A    ${LB_IP}

  Watch the certificate go ACTIVE (minutes after DNS resolves):

      gcloud compute ssl-certificates describe ${CERT} --global \\
        --format='value(managed.status,managed.domainStatus)'

  Verify once it is live:

      curl -sI https://${DOMAIN}/ | head -20
      curl -s  -o /dev/null -w '%{http_code}\\n' https://${DOMAIN}/spec/v1.0/concepts/   # 200
      curl -s  -o /dev/null -w '%{http_code}\\n' https://${DOMAIN}/nope/                 # 404, NOT 200

  Fast path for a content-only change:

      bash deploy/deploy_harnessxml.sh --content

EOF
