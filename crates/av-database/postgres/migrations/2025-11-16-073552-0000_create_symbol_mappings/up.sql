-- Symbol Mappings Table
-- Maps internal symbols (sid) to source-specific identifiers
CREATE TABLE symbol_mappings (
  id SERIAL PRIMARY KEY,
  sid BIGINT NOT NULL REFERENCES symbols(sid) ON DELETE CASCADE,
  source_name TEXT NOT NULL,
  source_identifier TEXT NOT NULL,
  verified BOOLEAN DEFAULT FALSE,
  last_verified_at TIMESTAMP,
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW(),
  UNIQUE(sid, source_name),
  UNIQUE(source_name, source_identifier)
);

CREATE INDEX idx_symbol_mappings_sid ON symbol_mappings(sid);
CREATE INDEX idx_symbol_mappings_source_name ON symbol_mappings(source_name);
CREATE INDEX idx_symbol_mappings_source_lookup ON symbol_mappings(source_name, source_identifier);
CREATE INDEX idx_symbol_mappings_verified ON symbol_mappings(verified) WHERE verified = TRUE;

-- Trigger function to update updated_at
CREATE OR REPLACE FUNCTION update_symbol_mappings_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_symbol_mappings_updated_at
  BEFORE UPDATE ON symbol_mappings
  FOR EACH ROW
  EXECUTE FUNCTION update_symbol_mappings_updated_at();
