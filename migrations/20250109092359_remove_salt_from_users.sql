-- migrations/20250109092359_remove_salt_from_users.sql
ALTER TABLE users DROP COLUMN salt;
