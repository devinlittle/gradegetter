CREATE TABLE schoology_auth (
    id UUID PRIMARY KEY,  -- User's service uuid
    encrypted_email TEXT NOT NULL,   -- User's schoology google email
    encrypted_password TEXT NOT NULL,-- User's schoology google password
    session_token TEXT,              -- User's schoology session token
    FOREIGN KEY (id) REFERENCES service_auth(id) ON DELETE CASCADE
);
