-- We use TEXT for ciphertext storage as we base64 encoded it.
ALTER TABLE ctc_records
    ADD COLUMN encrypted_components TEXT,
    ADD COLUMN key_version VARCHAR(50),
    ADD COLUMN encryption_version VARCHAR(50),
    ADD COLUMN encryption_algorithm VARCHAR(100),
    ADD COLUMN encrypted_at TIMESTAMPTZ;

-- Encryption metadata must be either fully absent (pre-backfill rows)
-- or fully present and internally consistent (post-backfill/new rows).
ALTER TABLE ctc_records
    ADD CONSTRAINT chk_ctc_encryption_metadata_consistent
    CHECK (
        (
            encrypted_components IS NULL
            AND key_version IS NULL
            AND encryption_version IS NULL
            AND encryption_algorithm IS NULL
            AND encrypted_at IS NULL
        )
        OR
        (
            encrypted_components IS NOT NULL
            AND key_version IS NOT NULL
            AND encryption_version IS NOT NULL
            AND encryption_algorithm IS NOT NULL
            AND encrypted_at IS NOT NULL
        )
    );

-- NOTE: plaintext component columns are intentionally kept during foundation rollout
-- to support controlled backfill and rollback safety. They should be removed only in
-- a dedicated post-backfill cutover migration once integrity verification is complete.

-- Non-sensitive operational columns (like resource_id, effective_date, status, working_days_per_month) remain plaintext
-- because they are needed for joins/filtering (e.g. blended rates) and status management.
