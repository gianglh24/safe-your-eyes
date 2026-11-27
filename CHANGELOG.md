# Changelog

All notable changes to Safe Your Eyes are recorded in this file.

## [0.5.0] - 2026-11-27

### Added

- Multi-language support: the app now supports English and Vietnamese (`language` setting: `system`, `en`, or `vi`) across the Settings window, break overlay, pre-break banner, and system tray menu.
- Microsoft Store build (`--features msstore`): Windows MSIX Store packages disable in-app update checks and binary replacement so Microsoft Store manages updates.

## [0.4.0] - 2026-11-27

### Added

- Windows: release builds now make a Microsoft Store (MSIX) bundle for x64 and ARM64. The Store build uses the Windows Startup Apps setting for autostart. The Store build needs Windows 10 version 2004 or later.
- Do Not Disturb: the app detects game sessions and does not show breaks while you play. The `do_not_disturb.while_gaming` setting controls this and is on by default.
- The pre-break banner shows a progress bar and a remaining-time badge. The banner color changes when the break is near.

### Changed

- Autostart is on by default for new installs.
- Screen readers announce the countdown in whole minutes, so the label does not change every second.

### Fixed

- macOS: dragging the Settings window no longer makes the window jump.
- Linux: the app restarts correctly after a self-update.
- A broken `config.toml` no longer changes the system autostart setting.
