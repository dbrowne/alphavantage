-- This file should undo anything in `up.sql`
drop TABLE IF EXISTS  article_translations ;
drop TABLE IF EXISTS  article_media ;
-- Tags
drop TABLE IF EXISTS  article_tags ;
-- Multiple symbol associations
drop TABLE IF EXISTS  article_symbols ;
-- Quote/Retweet information
drop TABLE IF EXISTS  article_quotes ;
-- Extend articles table
drop TABLE IF EXISTS  crypto_metadata ;
