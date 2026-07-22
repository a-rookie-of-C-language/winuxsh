# Minimal Winuxsh Configuration

Use this reference only when the user asks to configure Winuxsh. Keep edits
small and reversible.

Winuxsh uses `~/.winshrc.toml`.

## Small Example

```toml
[shell]
prompt_format = "{user}@{host} {cwd} {git_prompt}{symbol}"
prompt_symbol = ">"
right_prompt_format = "{time} "

[editor]
edit_mode = "emacs"

[aliases]
ll = "ls -la"

[completions]
matching = "prefix"
case_sensitive = false
```

Common changes:

- Set `edit_mode = "vi"` for vi-style editing.
- Set `matching = "substring"` for looser completions.
- Set `right_prompt_format = ""` to disable the right prompt.
- Add user aliases under `[aliases]`.

## WinuxCmd Override

Only add this when auto-discovery does not find the intended WinuxCmd:

```toml
[winuxcmd]
path = "D:/tools/winuxcmd/winuxcmd.exe"
```

## Zsh Migration

Use report/plan/apply/status/rollback flow. Do not source arbitrary zsh files.

```powershell
winuxsh --zsh-compat-report
winuxsh --zsh-compat-import-plan
winuxsh --zsh-compat-import-apply
winuxsh --zsh-compat-import-status
winuxsh --zsh-compat-import-rollback-plan
```

For built-in native zsh-style features:

```powershell
winuxsh --zsh-native-packs
winuxsh --zsh-profile-plan zsh-lite
winuxsh --zsh-profile-plan agent
```
