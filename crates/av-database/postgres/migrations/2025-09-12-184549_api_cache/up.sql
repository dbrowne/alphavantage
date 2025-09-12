-- Your SQL goes here
CREATE TABLE api_response_cache (
                                    cache_key VARCHAR(255) PRIMARY KEY,
                                    api_source VARCHAR(50) NOT NULL,
                                    endpoint_url TEXT NOT NULL,
                                    response_data JSONB NOT NULL,
                                    response_headers JSONB,
                                    status_code INTEGER NOT NULL,
                                    cached_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                                    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
                                    etag VARCHAR(255),
                                    last_modified VARCHAR(255)
);

CREATE INDEX idx_api_cache_source_expires ON api_response_cache(api_source, expires_at);
CREATE INDEX idx_api_cache_expires ON api_response_cache(expires_at);