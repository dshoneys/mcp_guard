# GitHub 仓库设置（建议，需 lead 在网页上点一次）

## Branch protection (`master`)

- Restrict who can push: 仅 lead / admin
- Require PR for others（隐式：无写权限则只能 fork/PR）
- Require status check: `adl-check` / `adl`
- **Administrators / lead 可绕过**（满足「最高权限者可直推 master」）

若团队用「全员有 write」：则必须开  
`Restrict pushes that create files that match…` 或规则「仅 admin 推 master」，并禁止 module 账号的 master 写权限。

## CODEOWNERS

已配置 `.github/CODEOWNERS`（`@shinjiyu`）。若 GitHub 用户名不同，改掉该文件。
