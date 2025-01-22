-- migrations/20250122170827_seed_user.sql
INSERT INTO users (user_id, username, password_hash)
VALUES (
    'dc3ae6c2-d8db-405e-b677-dd1ba568e15d',
    'admin',
    '$argon2id$v=19$m=15000,t=2,p=1$MsDyjFIrj5wa8j2UmFuIPA$ustGZSPdhRIbFfG/xQogzyM80vIGD/sVixfYHPhgkzg'
)

