#!/usr/bin/env bash
set -euo pipefail

SKIP_QUALITY=false
SKIP_BACKEND_BUILD=false
SKIP_FRONTEND_BUILD=false

for argument in "$@"; do
    case "$argument" in
        --skip-quality) SKIP_QUALITY=true ;;
        --skip-backend-build) SKIP_BACKEND_BUILD=true ;;
        --skip-frontend-build) SKIP_FRONTEND_BUILD=true ;;
        --build-all)
            SKIP_BACKEND_BUILD=false
            SKIP_FRONTEND_BUILD=false
            ;;
        *)
            echo "Unknown argument: $argument" >&2
            exit 2
            ;;
    esac
done

branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" != "main" ]; then
    echo "Must deploy from main, currently on '$branch'" >&2
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo "Working tree must be clean before deploy" >&2
    exit 1
fi

if [ "$SKIP_QUALITY" = false ]; then
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo test --workspace --test export_bindings
    pnpm install --frozen-lockfile
    pnpm --dir frontend build
    pnpm --dir frontend typecheck
    pnpm --dir frontend lint
    pnpm --dir frontend knip
fi

tag_name="deploy_cadence_$(date -u +'%Y/%m/%d_%Hh%Mm%Ss')"
git tag "$tag_name"
git push origin "$tag_name"

gh workflow run deploy.yml \
    --ref main \
    -f skip_backend_build="$SKIP_BACKEND_BUILD" \
    -f skip_frontend_build="$SKIP_FRONTEND_BUILD" \
    -f confirm_deploy=true

echo "Deploy triggered: $tag_name"
echo "Track: $(gh repo view --json nameWithOwner -q .nameWithOwner)/actions"
