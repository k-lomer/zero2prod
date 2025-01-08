-- migrations/20250108103731_create_users_table.sql
-- Create Users Table
CREATE TABLE users(
    user_id uuid PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
);
