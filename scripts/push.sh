#!/bin/bash
set -euo pipefail

# push.sh: Push current branch and create/update PR with workflow watching
#
# Pushes the current feature branch to origin and creates a PR if it doesn't exist.
# Watches for PR checks and workflows to complete.

# Colors for output
RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
BOLD=$'\033[1m'
GRAY=$'\033[0;90m'
WHITE=$'\033[1;37m'
NC=$'\033[0m' # No Color
SUCCESS=$'\033[1;32m✅'
FAIL=$'\033[1;31m❌'
SPINNER=$'\033[1;33m⏳'

# Parse arguments
SHOULD_MERGE=false
if [[ "${1:-}" == "--merge" ]] || [[ "${1:-}" == "-m" ]]; then
  SHOULD_MERGE=true
fi

# Main function - orchestrates the entire push workflow
main() {
  # Check we're in a git repo
  if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}Error: Not in a git repository${NC}"
    exit 1
  fi

  # Change to repo root
  cd "$(git rev-parse --show-toplevel)"

  # Check required tools
  if ! command -v gh &> /dev/null; then
    echo -e "${RED}Error: 'gh' command not found. Please install GitHub CLI.${NC}"
    exit 1
  fi
  if ! command -v jq &> /dev/null; then
    echo -e "${RED}Error: 'jq' command not found. Please install jq.${NC}"
    exit 1
  fi

  # Get current branch and repo info
  local current_branch
  current_branch="$(git rev-parse --abbrev-ref HEAD)"

  # Validate feature branch
  check_feature_branch "$current_branch"

  # Get repo URL for branch link
  local repo_url
  repo_url="$(git remote get-url origin 2>/dev/null | sed 's/\.git$//' | sed 's/^git@github\.com:/https:\/\/github.com\//' || echo "")"

  # Push changes
  push_changes "$current_branch" "$repo_url"

  # Ensure PR exists right after pushing
  local pr_number pr_url pr_title
  pr_url="$(ensure_pr "$current_branch")"
  pr_number="$(extract_pr_number "$pr_url")"
  
  if [[ -n "$pr_number" ]]; then
    pr_title="$(gh pr view "$pr_number" --json title --jq -r '.title' 2>/dev/null || echo "")"
  fi

  # Wait for validation to complete
  if await_validation "$current_branch" "$pr_url"; then
    if [[ "$SHOULD_MERGE" == "true" ]]; then
      if [[ -n "$pr_number" ]]; then
        merge_pr "$pr_number"
      else
        echo -e "${YELLOW}Warning: Cannot merge PR without PR number${NC}"
      fi
    else
      open_pr "$pr_url"
    fi
  else
    echo -e "${RED}Build failed. Check the logs above for details.${NC}"
    exit 1
  fi
}

# Check that we're on a feature branch
check_feature_branch() {
  local branch="$1"
  if [[ ! "$branch" =~ ^feature/ ]]; then
    echo -e "${RED}Error: Not on a feature branch. Current branch is: $branch${NC}"
    echo "Please switch to a feature/* branch before running this script."
    exit 1
  fi
}

# Push changes with force-with-lease
push_changes() {
  local branch="$1"
  local repo_url="$2"
  
  local branch_url=""
  if [[ -n "$repo_url" ]]; then
    branch_url="$repo_url/tree/$branch"
  fi
  
  printf "%s%s Pushing branch... " "${SPINNER}" "${NC}"
  
  if git push --force-with-lease origin "$branch" > /dev/null 2>&1; then
    printf "\r%s%s Pushed branch: %s%s%s\n" "${SUCCESS}" "${NC}" "${GRAY}" "$branch_url" "${NC}"
  else
    printf "\r%s%s Failed to push branch\n" "${FAIL}" "${NC}"
    git push --force-with-lease origin "$branch"
    exit 1
  fi
}

# Ensure PR exists, create if needed, return PR URL
ensure_pr() {
  local branch="$1"
  local base_branch="main"

  # Check if PR already exists
  local existing_pr pr_info
  existing_pr="$(gh pr list --head "$branch" --json number,url,title --jq '.[0]' 2>/dev/null || echo "")"

  if [[ -n "$existing_pr" ]] && [[ "$existing_pr" != "null" ]]; then
    local pr_url pr_num pr_title
    pr_url="$(echo "$existing_pr" | jq -r '.url')"
    pr_num="$(echo "$existing_pr" | jq -r '.number')"
    pr_title="$(echo "$existing_pr" | jq -r '.title')"
    
    printf "%s%s Ensuring PR... " "${SPINNER}" "${NC}"
    printf "\r%s%s Ensuring PR: @PR %s: %s%s%s\n" "${SUCCESS}" "${NC}" "$pr_num" "${WHITE}" "$pr_title" "${NC}"
    echo "$pr_url"
    return
  fi

  # Create new PR
  printf "%s%s Ensuring PR... " "${SPINNER}" "${NC}"
  local pr_url
  pr_url="$(gh pr create --head "$branch" --base "$base_branch" --title "WIP: $branch" --body "Automated PR for $branch" --draft 2>/dev/null || true)"

  if [[ -z "$pr_url" ]]; then
    printf "\r%s%s Failed to create PR\n" "${FAIL}" "${NC}"
    exit 1
  fi

  local pr_num pr_title
  pr_num="$(extract_pr_number "$pr_url")"
  if [[ -n "$pr_num" ]]; then
    pr_title="$(gh pr view "$pr_num" --json title --jq -r '.title' 2>/dev/null || echo "")"
    printf "\r%s%s Ensuring PR: @PR %s: %s%s%s\n" "${SUCCESS}" "${NC}" "$pr_num" "${WHITE}" "$pr_title" "${NC}"
  else
    printf "\r%s%s Ensuring PR\n" "${SUCCESS}" "${NC}"
  fi
  
  echo "$pr_url"
}

# Extract PR number from URL
extract_pr_number() {
  local url="$1"
  if [[ "$url" =~ /pull/([0-9]+) ]]; then
    echo "${BASH_REMATCH[1]}"
  fi
}

# Wait for validation (workflows/checks) to complete
# Returns 0 (success) if successful, 1 (failure) if failed
await_validation() {
  local branch="$1"
  local pr_url="$2"

  printf "%s%s Awaiting build checks... " "${SPINNER}" "${NC}"

  # Get the commit hash
  local commit_hash
  commit_hash="$(git rev-parse HEAD)"

  # First, wait for workflow to start (if it hasn't already)
  local run_id run_url
  run_id="$(wait_for_workflow_start "$commit_hash")"

  if [[ -z "$run_id" ]]; then
    printf "\r%s%s No workflow found for commit\n" "${FAIL}" "${NC}"
    printf "%sWarning: No workflow found for commit %s%s\n" "${YELLOW}" "$commit_hash" "${NC}"
    echo "This might mean workflows haven't started yet, or no workflows are configured."
    return 1
  fi

  # Get run URL for display
  run_url="$(gh run view "$run_id" --json htmlUrl --jq -r '.htmlUrl' 2>/dev/null || echo "")"
  if [[ -z "$run_url" ]]; then
    # Fallback: construct URL from repo info
    local repo_owner repo_name
    repo_owner="$(gh repo view --json owner --jq -r '.owner.login' 2>/dev/null || echo "")"
    repo_name="$(gh repo view --json name --jq -r '.name' 2>/dev/null || echo "")"
    if [[ -n "$repo_owner" ]] && [[ -n "$repo_name" ]]; then
      run_url="https://github.com/$repo_owner/$repo_name/actions/runs/$run_id"
    fi
  fi

  # Check if workflow already completed (idempotent check)
  local run_info
  run_info="$(gh run view "$run_id" --json status,conclusion 2>/dev/null || echo "")"
  
  if [[ -n "$run_info" ]]; then
    local status conclusion
    status="$(echo "$run_info" | jq -r '.status // "unknown"')"
    conclusion="$(echo "$run_info" | jq -r '.conclusion // "none"')"

    if [[ "$status" == "completed" ]] && [[ "$conclusion" == "success" ]]; then
      printf "\r%s%s Build checks passed\n" "${SUCCESS}" "${NC}"
      if [[ -n "$run_url" ]]; then
        printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
      fi
      return 0
    elif [[ "$status" == "completed" ]] && ([[ "$conclusion" == "failure" ]] || [[ "$conclusion" == "cancelled" ]]); then
      printf "\r%s%s Build checks failed\n" "${FAIL}" "${NC}"
      if [[ -n "$run_url" ]]; then
        printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
      fi
      handle_workflow_failure "$run_id" "$run_url" "$pr_url"
      return 1
    elif [[ "$conclusion" == "failure" ]] || [[ "$conclusion" == "cancelled" ]]; then
      # Conclusion set but status might not be "completed" yet
      printf "\r%s%s Build checks failed\n" "${FAIL}" "${NC}"
      if [[ -n "$run_url" ]]; then
        printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
      fi
      handle_workflow_failure "$run_id" "$run_url" "$pr_url"
      return 1
    fi
  fi

  # Watch workflow until completion
  watch_workflow "$run_id" "$run_url" "$pr_url"
}

# Wait for workflow to start, return run ID
# Gets the most recent run for the commit
wait_for_workflow_start() {
  local commit_hash="$1"
  local max_wait=60  # 60 seconds
  local elapsed=0
  local interval=2   # Check every 2 seconds

  while [[ $elapsed -lt $max_wait ]]; do
    # Get the most recent run (gh run list returns runs sorted by most recent first by default)
    # Use databaseId for the run ID (this is the numeric ID used in URLs)
    local run_id
    run_id="$(gh run list --commit "$commit_hash" --limit 1 --json databaseId --jq '.[0].databaseId // empty' 2>/dev/null || true)"

    if [[ -n "$run_id" ]] && [[ "$run_id" != "null" ]] && [[ "$run_id" != "" ]]; then
      echo "$run_id"
      return 0
    fi

    sleep "$interval"
    elapsed=$((elapsed + interval))
  done

  return 1
}

# Watch workflow until completion
# Returns 0 (success) if successful, 1 (failure) if failed
watch_workflow() {
  local run_id="$1"
  local run_url="$2"
  local pr_url="$3"
  local max_wait=1800  # 30 minutes
  local elapsed=0
  local interval=5     # Check every 5 seconds
  local consecutive_failures=0
  local max_consecutive_failures=5

  while [[ $elapsed -lt $max_wait ]]; do
    # Get both status and conclusion in one call for efficiency
    local run_info
    run_info="$(gh run view "$run_id" --json status,conclusion 2>/dev/null || echo "")"
    
    if [[ -z "$run_info" ]]; then
      consecutive_failures=$((consecutive_failures + 1))
      if [[ $consecutive_failures -ge $max_consecutive_failures ]]; then
        echo
        printf "%sError: Could not fetch workflow status after %s attempts%s\n" "${RED}" "$max_consecutive_failures" "${NC}"
        if [[ -n "$run_url" ]]; then
          printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
        else
          printf "Run ID: %s\n" "$run_id"
        fi
        return 1
      fi
      echo
      printf "%sWarning: Could not fetch workflow status. Retrying... (attempt %s/%s)%s\n" "${YELLOW}" "$consecutive_failures" "$max_consecutive_failures" "${NC}"
      sleep "$interval"
      elapsed=$((elapsed + interval))
      continue
    fi
    
    # Reset failure counter on success
    consecutive_failures=0

    local status conclusion
    status="$(echo "$run_info" | jq -r '.status // "unknown"')"
    conclusion="$(echo "$run_info" | jq -r '.conclusion // "none"')"

    # Check if workflow is completed
    if [[ "$status" == "completed" ]]; then
      if [[ "$conclusion" == "success" ]]; then
        printf "\r%s%s Build checks passed\n" "${SUCCESS}" "${NC}"
        if [[ -n "$run_url" ]]; then
          printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
        fi
        return 0
      else
        printf "\r%s%s Build checks failed\n" "${FAIL}" "${NC}"
        if [[ -n "$run_url" ]]; then
          printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
        fi
        handle_workflow_failure "$run_id" "$run_url" "$pr_url"
        return 1
      fi
    fi

    # Also check conclusion even if status isn't "completed" yet
    # Sometimes conclusion is set before status becomes "completed"
    if [[ "$conclusion" != "none" ]] && [[ "$conclusion" != "null" ]]; then
      if [[ "$conclusion" == "success" ]]; then
        printf "\r%s%s Build checks passed\n" "${SUCCESS}" "${NC}"
        if [[ -n "$run_url" ]]; then
          printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
        fi
        return 0
      elif [[ "$conclusion" == "failure" ]] || [[ "$conclusion" == "cancelled" ]]; then
        printf "\r%s%s Build checks failed\n" "${FAIL}" "${NC}"
        if [[ -n "$run_url" ]]; then
          printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
        fi
        handle_workflow_failure "$run_id" "$run_url" "$pr_url"
        return 1
      fi
    fi

    # Show progress (only every 30 seconds to keep it quiet)
    if [[ $((elapsed % 30)) -eq 0 ]] && [[ $elapsed -gt 0 ]]; then
      echo -n "."
    fi

    sleep "$interval"
    elapsed=$((elapsed + interval))
  done

  echo
  printf "%sTimeout waiting for workflow to complete%s\n" "${YELLOW}" "${NC}"
  if [[ -n "$run_url" ]]; then
    printf "Run: %s%s%s\n" "${GRAY}" "$run_url" "${NC}"
  fi
  return 1
}

# Handle workflow failure - download logs and analyze
handle_workflow_failure() {
  local run_id="$1"
  local run_url="$2"
  local pr_url="$3"

  echo
  printf "%sDownloading logs...%s\n" "${RED}" "${NC}"

  # Create artifacts directory
  local repo_root
  repo_root="$(git rev-parse --show-toplevel)"
  local artifacts_dir="$repo_root/.ci-artifacts/$run_id"
  mkdir -p "$artifacts_dir"

  # Check if logs already exist (idempotent)
  local log_file="$artifacts_dir/build.log"
  if [[ ! -f "$log_file" ]]; then
    if ! gh run view "$run_id" --log > "$log_file" 2>&1; then
      # Try alternative method
      local zip_file="$artifacts_dir/workflow-logs.zip"
      if gh api "repos/:owner/:repo/actions/runs/$run_id/logs" > "$zip_file" 2>/dev/null; then
        local logs_dir="$artifacts_dir/logs"
        mkdir -p "$logs_dir"
        unzip -o "$zip_file" -d "$logs_dir" > /dev/null 2>&1 || true
        rm -f "$zip_file"
        log_file="$logs_dir"
      else
        log_file=""
      fi
    fi
  fi

  # Analyze logs for errors and warnings
  if [[ -n "$log_file" ]] && [[ -e "$log_file" ]]; then
    echo
    grep_log_failures "$log_file"
    echo
    printf "Logs: %s%s%s\n" "${GRAY}" "$log_file" "${NC}"
  fi
}

# Grep logs for common error and warning patterns
grep_log_failures() {
  local log_path="$1"
  local pattern='(error|failed|failure|exception|fatal|✖|FAIL|ERROR|Error:|Failed:|warning|Warning:|WARNING)'

  local results
  if [[ -d "$log_path" ]]; then
    # Search recursively in directory
    results="$(grep -r -i -E "$pattern" "$log_path" 2>/dev/null | head -30 || true)"
  else
    # Search in single file
    results="$(grep -i -E "$pattern" "$log_path" 2>/dev/null | head -30 || true)"
  fi

  if [[ -n "$results" ]]; then
    printf "%sErrors/warnings found:%s\n" "${RED}" "${NC}"
    echo "$results" | while IFS= read -r line; do
      # Truncate very long lines
      if [[ ${#line} -gt 200 ]]; then
        printf "  %s%s...%s\n" "${GRAY}" "${line:0:200}" "${NC}"
      else
        printf "  %s%s%s\n" "${GRAY}" "$line" "${NC}"
      fi
    done
  fi
}

# Merge PR
merge_pr() {
  local pr_number="$1"
  printf "%s%s Merging PR #%s... " "${SPINNER}" "${NC}" "$pr_number"
  if gh pr merge "$pr_number" --merge --delete-branch=false > /dev/null 2>&1; then
    printf "\r%s%s Merged PR #%s\n" "${SUCCESS}" "${NC}" "$pr_number"
  else
    printf "\r%s%s Failed to merge PR #%s\n" "${FAIL}" "${NC}" "$pr_number"
    gh pr merge "$pr_number" --merge --delete-branch=false
    exit 1
  fi
}

# Open PR in browser
open_pr() {
  local pr_url="$1"
  open "$pr_url" > /dev/null 2>&1
}

# Run main function
main "$@"
