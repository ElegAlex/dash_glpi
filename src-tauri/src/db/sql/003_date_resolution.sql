ALTER TABLE tickets ADD COLUMN date_resolution TEXT;
CREATE INDEX IF NOT EXISTS idx_tickets_date_resolution ON tickets(import_id, date_resolution);
