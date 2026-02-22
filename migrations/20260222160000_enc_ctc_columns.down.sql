ALTER TABLE ctc_records 
    DROP CONSTRAINT IF EXISTS chk_ctc_encryption_metadata_consistent,
    DROP COLUMN IF EXISTS encrypted_components,
    DROP COLUMN IF EXISTS key_version,
    DROP COLUMN IF EXISTS encryption_version,
    DROP COLUMN IF EXISTS encryption_algorithm,
    DROP COLUMN IF EXISTS encrypted_at;
