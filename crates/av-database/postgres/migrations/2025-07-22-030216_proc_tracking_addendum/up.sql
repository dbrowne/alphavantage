-- Check and create tables only if they don't exist

-- Process types table
CREATE TABLE IF NOT EXISTS proctypes (
                                         id   SERIAL PRIMARY KEY,
                                         name TEXT NOT NULL UNIQUE
);

-- Insert process types, ignoring conflicts
INSERT INTO proctypes (name) VALUES
                                 ('load_symbols'),
                                 ('load_overviews'),
                                 ('load_intraday'),
                                 ('load_summary'),
                                 ('load_topstats'),
                                 ('load_news'),
                                 ('calculate_sentiment')
    ON CONFLICT (name) DO NOTHING;

-- States table
CREATE TABLE IF NOT EXISTS states (
                                      id   SERIAL PRIMARY KEY,
                                      name TEXT NOT NULL UNIQUE
);

-- Insert states, ignoring conflicts
INSERT INTO states (name) VALUES
                              ('started'),
                              ('completed'),
                              ('failed'),
                              ('cancelled'),
                              ('retrying')
    ON CONFLICT (name) DO NOTHING;

-- Process states table
CREATE TABLE IF NOT EXISTS procstates (
                                          spid       SERIAL PRIMARY KEY,
                                          proc_id    INTEGER REFERENCES proctypes(id),
    start_time TIMESTAMP NOT NULL DEFAULT NOW(),
    end_state  INTEGER REFERENCES states(id),
    end_time   TIMESTAMP,
    error_msg  TEXT,
    records_processed INTEGER DEFAULT 0
    );

-- Create indexes if they don't exist
CREATE INDEX IF NOT EXISTS idx_procstates_proc_id ON procstates(proc_id);
CREATE INDEX IF NOT EXISTS idx_procstates_start_time ON procstates(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_procstates_end_state ON procstates(end_state);

-- Drop trigger if exists before recreating
DROP TRIGGER IF EXISTS prevent_procstate_update ON procstates;

-- Function with CREATE OR REPLACE handles existing function
CREATE OR REPLACE FUNCTION prevent_completed_update()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.end_state = 2 AND OLD.end_state IS NOT NULL THEN
        RAISE EXCEPTION 'Cannot update completed process %', OLD.spid;
END IF;
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Recreate trigger
CREATE TRIGGER prevent_procstate_update
    BEFORE UPDATE ON procstates
    FOR EACH ROW
    EXECUTE FUNCTION prevent_completed_update();
