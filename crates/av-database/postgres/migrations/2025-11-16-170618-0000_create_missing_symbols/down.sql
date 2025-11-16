-- Drop the missing_symbols table and related objects
DROP TRIGGER IF EXISTS trigger_missing_symbols_updated_at ON missing_symbols;
DROP FUNCTION IF EXISTS update_missing_symbols_updated_at();
DROP TABLE IF EXISTS missing_symbols;