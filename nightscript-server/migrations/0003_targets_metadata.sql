ALTER TABLE packages
    ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';

ALTER TABLE versions
    ADD COLUMN targets_json TEXT NOT NULL DEFAULT '{}';
