-- Table to track symbols that are missing from the database
-- Used to identify symbols encountered in news feeds that need to be loaded
CREATE TABLE missing_symbols (
  id SERIAL PRIMARY KEY,
  symbol TEXT NOT NULL,
  source TEXT NOT NULL,  -- Where the symbol was encountered (e.g., 'news_feed', 'api_call')
  first_seen_at TIMESTAMP NOT NULL DEFAULT NOW(),
  last_seen_at TIMESTAMP NOT NULL DEFAULT NOW(),
  seen_count INTEGER NOT NULL DEFAULT 1,
  resolution_status TEXT NOT NULL DEFAULT 'pending',  -- 'pending', 'found', 'not_found', 'skipped'
  sid BIGINT,  -- Set when symbol is found and loaded into symbols table
  resolution_details TEXT,  -- Additional info about resolution attempt
  resolved_at TIMESTAMP,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
  UNIQUE(symbol, source)
);

-- Index for quick lookups
CREATE INDEX idx_missing_symbols_symbol ON missing_symbols(symbol);
CREATE INDEX idx_missing_symbols_status ON missing_symbols(resolution_status);
CREATE INDEX idx_missing_symbols_first_seen ON missing_symbols(first_seen_at DESC);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_missing_symbols_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_missing_symbols_updated_at
  BEFORE UPDATE ON missing_symbols
  FOR EACH ROW
  EXECUTE FUNCTION update_missing_symbols_updated_at();

-- Add comment explaining the table
COMMENT ON TABLE missing_symbols IS 'Tracks symbols encountered in data feeds that are not yet in the symbols table';
COMMENT ON COLUMN missing_symbols.resolution_status IS 'Status: pending (not yet attempted), found (loaded successfully), not_found (does not exist), skipped (intentionally not loaded)';
