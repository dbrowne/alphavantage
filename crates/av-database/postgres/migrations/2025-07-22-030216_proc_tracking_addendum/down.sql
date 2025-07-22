-- This file should undo anything in `up.sql`
DROP TRIGGER IF EXISTS prevent_procstate_update ON procstates;
DROP FUNCTION IF EXISTS prevent_completed_update();
DROP TABLE IF EXISTS procstates;
DROP TABLE IF EXISTS states;
DROP TABLE IF EXISTS proctypes;