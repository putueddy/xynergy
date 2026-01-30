-- Update admin password with proper Argon2 hash
-- Password: admin123
-- Hash generated with Argon2id

UPDATE users 
SET password_hash = '$argon2id$v=19$m=19456,t=2,p=1$SDun6cPGLz3nKIm9mOITmQ$f3ejueiCxxCjBrsiBQApkUJOBJMEUVmbSfumhZC2igM'
WHERE email = 'admin@xynergy.com';
