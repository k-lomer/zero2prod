-- migrations/20250108115634_rename_password_column.sql
ALTER TABLE users RENAME password TO password_hash;
