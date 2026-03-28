#!/usr/bin/env bash
set -euo pipefail

pr=""
repo=""

usage() {
  cat <<'USAGE'
Usage: triage-pr.sh [--pr <number>] [--repo <owner/repo>]

Prints a single-shot summary of CI status, latest review, and latest comments.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pr)
      pr="$2"
      shift 2
      ;;
    --repo)
      repo="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$pr" ]]; then
  pr="$(gh pr view --json number --jq .number 2>/dev/null || true)"
fi

if [[ -z "$pr" ]]; then
  echo "Could not determine PR number. Use --pr <number>." >&2
  exit 1
fi

if [[ -z "$repo" ]]; then
  repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null || true)"
fi

if [[ -z "$repo" ]]; then
  echo "Could not determine repo. Use --repo owner/name." >&2
  exit 1
fi

total=0
pending=0
failed=0
success=0
failed_checks=""

# Same rationale as poll-pr.sh: JSON mode exits non-zero on failed checks.
checks_output=$(gh pr checks "$pr" --repo "$repo" 2>/dev/null || true)
if [[ -n "$checks_output" ]]; then
  while IFS=$'\t' read -r check_name check_state _check_age check_url _rest; do
    if [[ -z "$check_name" || -z "$check_state" ]]; then
      continue
    fi

    total=$((total + 1))
    check_state_lc=$(echo "$check_state" | tr '[:upper:]' '[:lower:]')
    case "$check_state_lc" in
      pass|success)
        success=$((success + 1))
        ;;
      pending|in_progress|queued|requested|waiting)
        pending=$((pending + 1))
        ;;
      skip|skipped|neutral)
        ;;
      *)
        failed=$((failed + 1))
        failed_checks+="${check_name}"$'\t'"${check_state}"$'\t'"${check_url}"$'\n'
        ;;
    esac
  done <<< "$checks_output"

  echo "CI: total=$total pending=$pending failed=$failed success=$success"
else
  echo "CI: unavailable"
fi

if [[ -n "$failed_checks" ]]; then
  while IFS=$'\t' read -r name conclusion url; do
    if [[ -n "$name" ]]; then
      echo "FAIL: $name $conclusion $url"
    fi
  done <<< "$failed_checks"
fi

review_line=$(gh api "repos/$repo/pulls/$pr/reviews?per_page=100" --jq '
  [ .[] | select(.submitted_at != null) ] |
  if length == 0 then "" else
    (max_by(.submitted_at)) | "\(.state)\t\(.user.login)\t\(.submitted_at)\t\(.html_url)"
  end
' 2>/dev/null || true)
if [[ -n "$review_line" ]]; then
  IFS=$'\t' read -r r_state r_user r_time r_url <<< "$review_line"
  echo "REVIEW: $r_state $r_user $r_time $r_url"
fi

issue_line=$(gh api "repos/$repo/issues/$pr/comments?per_page=100" --jq '
  if length == 0 then "" else
    (max_by(.created_at)) | "\(.user.login)\t\(.created_at)\t\(.html_url)\t\(.body | gsub("\\n"; " ") | gsub("\\t"; " ") | .[0:200])"
  end
' 2>/dev/null || true)
if [[ -n "$issue_line" ]]; then
  IFS=$'\t' read -r c_user c_time c_url c_body <<< "$issue_line"
  echo "COMMENT: conversation $c_user $c_time $c_url $c_body"
fi

review_comment_line=$(gh api "repos/$repo/pulls/$pr/comments?per_page=100" --jq '
  if length == 0 then "" else
    (max_by(.created_at)) | "\(.user.login)\t\(.created_at)\t\(.html_url)\t\(.body | gsub("\\n"; " ") | gsub("\\t"; " ") | .[0:200])"
  end
' 2>/dev/null || true)
if [[ -n "$review_comment_line" ]]; then
  IFS=$'\t' read -r rc_user rc_time rc_url rc_body <<< "$review_comment_line"
  echo "COMMENT: inline $rc_user $rc_time $rc_url $rc_body"
fi
