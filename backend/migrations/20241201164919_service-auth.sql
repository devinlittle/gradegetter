-- Add migration script here
CREATE TABLE service_auth (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,      -- Unique user identifier
    username VARCHAR(50) NOT NULL UNIQUE,               -- Username for login
    password_hash TEXT NOT NULL,                        -- Hashed password
    created_at TIMESTAMP DEFAULT NOW()                  -- Account creation timestamp
);
