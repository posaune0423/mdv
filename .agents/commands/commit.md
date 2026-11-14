### Command: Commit current changes in logical groups (simple)

Do exactly this, non-interactively, from repo root.

1. Ignore when staging:
   - Follow .gitignore strictly. Additionally, ignore: .cursor/\*\* (except this file), .env

2. Define groups and scopes:
   - infra → config/**, composer.\* , package.json, vite.config.js, docker/**, docker-compose.yml
   - routes → routes/\*\*
   - controllers → app/Http/\*\*
   - models → app/Models/\*\*
   - views → resources/views/\*\*
   - frontend → resources/js/**, resources/css/**
   - migrations → database/migrations/\*\*
   - tests → tests/\*\*

3. For each group that has changes, stage and commit (by intent/responsibility, not only folder):
   - Decide values:
     - ${emoji}:{fix=🐛, feat=✨, docs=📝, style=💄, refactor=♻️, perf=🚀, test=💚, chore=🍱}
     - ${type} in {fix, feat, docs, style, refactor, perf, test, chore}
     - ${scope} = group name (e.g., routes|controllers|models|views|infra|frontend|migrations|tests)
     - ${summary} = 1-line imperative (<=72 chars)
     - ${body} = 1–3 bullets (optional)
   - Commands:
     - git add -A -- -- ${file1} ${file2} ${fileN}
     - git commit --no-verify --no-gpg-sign -m "${emoji} ${type}(${scope}): ${summary}" -m "${body}"

4. Commit order: chore → docs → style → refactor → perf → feat → fix → test

5. Final check:
   - git -c core.pager=cat status --porcelain=v1 | cat

Message template:
Title: "${emoji} ${type}(${scope}): ${summary}"
Body: "- ${changes}\n- ${reasonImpact}"

Example:
git add -A -- -- routes/admin.php app/Http/Controllers/Admin/UserController.php
git commit --no-verify --no-gpg-sign -m "✨ feat(routes): Add the admin user list" -m "- Implement the new route and index\n- Add permission checks"
