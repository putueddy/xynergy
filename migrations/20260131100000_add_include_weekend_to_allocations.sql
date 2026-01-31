-- Add include_weekend flag to allocations
ALTER TABLE allocations
    ADD COLUMN include_weekend BOOLEAN NOT NULL DEFAULT FALSE;
