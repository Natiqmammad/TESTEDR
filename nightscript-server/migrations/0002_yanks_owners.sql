ALTER TABLE versions ADD COLUMN yanked INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS package_owners (
    package_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    role TEXT NOT NULL DEFAULT 'owner',
    created_at TEXT NOT NULL,
    PRIMARY KEY(package_id, user_id),
    FOREIGN KEY(package_id) REFERENCES packages(id),
    FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_package_owners_package ON package_owners(package_id);
CREATE INDEX IF NOT EXISTS idx_package_owners_user ON package_owners(user_id);

INSERT OR IGNORE INTO package_owners (package_id, user_id, role, created_at)
SELECT id as package_id, owner_id as user_id, 'owner' as role, created_at
FROM packages;
