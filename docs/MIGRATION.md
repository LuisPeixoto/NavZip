# Importing an old Alpha profile

The original Servo/egui Alpha is retired. Its data-only export compatibility remains available so old profiles can be preserved without retaining a second browser implementation.

## Export a consistent backup

```sh
cargo xtask export-profile /path/to/alpha-profile /path/to/new-backup-directory
```

The source directory must contain the Alpha v1 `library.db`. The tool opens it read-only and uses SQLite backup to include an active WAL consistently. It refuses an existing destination, writes a database backup and versioned `profile.json`, and restricts destination permissions on Unix.

The JSON contains bookmarks, history, normal tabs and settings. It may contain sensitive browsing data: keep it private and do not attach it to issues. Cookies, cache and password-vault secrets are not exported. The old vault remains untouched.

## Import explicitly

```sh
cargo xtask run -- --navzip-import=/path/to/new-backup-directory/profile.json
```

Use a new, disposable Chromium/NavZip profile while testing the importer. Writes go through Chromium services rather than editing its database files directly. The implementation uses bookmark UUIDs and visit timestamps to avoid repeat entries, but complete native migration and crash-recovery qualification are pending. Do not delete the source profile or backup based only on a successful launch.

The retired application's cookies/cache are not compatible with Chromium profiles. No plaintext password-transfer file is created by this tool.
