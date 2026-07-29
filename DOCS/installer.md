# Installer and Self-Update

Winuxsh ships two Windows package shapes:

- `winuxsh-v<version>-win-<arch>-setup.exe` for normal users.
- `winuxsh-v<version>-win-<arch>.zip` for portable, agent, or scripted use.

The installer is built with Inno Setup and installs per user by default under:

```text
%LOCALAPPDATA%\Programs\Winuxsh
```

It does not require administrator privileges. The default installer tasks:

- add the install directory to the user's `PATH`;
- add or update a Windows Terminal profile named `Winuxsh`;
- set that profile's command line to the installed `winuxsh.exe`;
- set that profile's starting directory to `%USERPROFILE%`;
- point the Windows Terminal profile icon at the installed PNG asset.

The Windows Terminal profile is installed by running:

```sh
winuxsh --install-wt-profile --quiet
```

Users can run this command again after moving an install. To also set Winuxsh as
the Windows Terminal default profile, run:

```sh
winuxsh --install-wt-profile --set-default
```

Self-update uses Windows WinHTTP directly to follow the GitHub Release
`releases/latest` redirect, download the latest installer for the current
architecture, and start it silently. It does not depend on the GitHub REST API.

```sh
winuxsh --self-update
```

Useful dry-run modes:

```sh
winuxsh --self-update --check
winuxsh --self-update --dry-run
```

Interactive shells check for updates at most once per day. The check is
best-effort and silent on network failures; when a newer release exists, Winuxsh
prints a short hint to run `winuxsh --self-update`. Set
`WINUXSH_UPDATE_CHECK=0` or `WINUXSH_NO_UPDATE_CHECK=1` to disable the reminder.

The portable zip keeps the same first-start WinuxCmd activation flow: if command
links are missing, Winuxsh runs `winuxcmd/activate-winuxcmd.sh` once from the
bundle so `ls`, `cat`, `grep`, and friends resolve normally.
