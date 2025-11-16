-- Drop symbol_mappings table and related objects
DROP TRIGGER IF EXISTS trigger_symbol_mappings_updated_at ON symbol_mappings;
DROP FUNCTION IF EXISTS update_symbol_mappings_updated_at();
DROP TABLE IF EXISTS symbol_mappings;
