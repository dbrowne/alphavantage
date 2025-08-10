-- Your SQL goes here
-- Multi-language support
CREATE TABLE article_translations (
    id SERIAL PRIMARY KEY,
    articleid TEXT REFERENCES articles(hashid),
    language VARCHAR(10) NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    UNIQUE(articleid, language)
);
-- Media attachments
CREATE TABLE article_media (
    id SERIAL PRIMARY KEY,
    articleid TEXT REFERENCES articles(hashid),
    soso_url TEXT,
    original_url TEXT,
    short_url TEXT,
    media_type VARCHAR(20),
    media_order INTEGER DEFAULT 0
);
-- Tags
CREATE TABLE article_tags (
    articleid TEXT REFERENCES articles(hashid),
    tag VARCHAR(100),
    PRIMARY KEY (articleid, tag)
);
-- Multiple symbol associations
CREATE TABLE article_symbols (
    articleid TEXT REFERENCES articles(hashid),
    sid BIGINT REFERENCES symbols(sid),
    full_name TEXT,
    PRIMARY KEY (articleid, sid)
);
-- Quote/Retweet information
CREATE TABLE article_quotes (
    id SERIAL PRIMARY KEY,
    articleid TEXT REFERENCES articles(hashid),
    original_url TEXT,
    author VARCHAR(255),
    author_avatar_url TEXT,
    nick_name VARCHAR(255),
    impression_count BIGINT,
    like_count INTEGER,
    reply_count INTEGER,
    retweet_count INTEGER,
    twitter_created_at TIMESTAMPTZ
);
-- Extend articles table
ALTER TABLE articles 
ADD COLUMN source_link TEXT,
ADD COLUMN release_time BIGINT,
ADD COLUMN author_description TEXT,
ADD COLUMN author_avatar_url TEXT,
ADD COLUMN feature_image TEXT,
ADD COLUMN author_nick_name VARCHAR(255);

CREATE TABLE crypto_metadata (
    sid BIGINT PRIMARY KEY REFERENCES symbols(sid),
    source VARCHAR(50) NOT NULL,
    source_id TEXT NOT NULL,
    market_cap_rank INTEGER,
    base_currency VARCHAR(10),
    quote_currency VARCHAR(10),
    is_active BOOLEAN NOT NULL DEFAULT true,
    additional_data JSONB,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(source, source_id)
);

