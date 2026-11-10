# Configuration Transactions & Rollback

## Transaction model
Each integration change is a transaction:
1. Detect target app/version and config path.
2. Read original bytes, metadata and hash.
3. Parse using correct format.
4. Create timestamped backup in Hub data directory.
5. Build a minimal patch; show diff to user.
6. Write temporary file in same directory.
7. Flush and atomically replace.
8. Validate resulting config.
9. Run verification.
10. Commit transaction record or restore exact original bytes.

## Requirements
- Never overwrite unknown/unparseable config automatically.
- Preserve unrelated user keys.
- Backups immutable by default.
- Store original and resulting SHA-256.
- Rollback only if current file hash matches Hub's last applied hash; otherwise detect external edits and ask user before restoring.
- Support per-integration disconnect (remove only Hub-owned config).
- Provide “Restore all Hub changes” with preview.
- Keep audit trail: timestamp, app, config path, change summary, verification result.
