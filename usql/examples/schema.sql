CREATE TABLE
  user (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL
  );

CREATE INDEX idx_user_email ON user (email);

CREATE TABLE
  blog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user (id) ON DELETE CASCADE
  );

CREATE TABLE
  comment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    blog_id INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user (id) ON DELETE CASCADE,
    FOREIGN KEY (blog_id) REFERENCES blog (id) ON DELETE CASCADE
  );

INSERT INTO
  user (name, email)
VALUES
  ('Alice', 'alice@example.com'),
  ('Bob', 'bob@example.com');

INSERT INTO
  blog (title, content, user_id)
VALUES
  (
    'Alice\'s Blog 1',
    'Content for Alice\'s first blog.',
    1
  ),
  (
    'Alice\'s Blog 2',
    'Content for Alice\'s second blog.',
    1
  ),
  (
    'Alice\'s Blog 3',
    'Content for Alice\'s third blog.',
    1
  ),
  (
    'Alice\'s Blog 4',
    'Content for Alice\'s fourth blog.',
    1
  ),
  (
    'Alice\'s Blog 5',
    'Content for Alice\'s fifth blog.',
    1
  ),
  (
    'Bob\'s Blog 1',
    'Content for Bob\'s first blog.',
    2
  ),
  (
    'Bob\'s Blog 2',
    'Content for Bob\'s second blog.',
    2
  ),
  (
    'Bob\'s Blog 3',
    'Content for Bob\'s third blog.',
    2
  ),
  (
    'Bob\'s Blog 4',
    'Content for Bob\'s fourth blog.',
    2
  ),
  (
    'Bob\'s Blog 5',
    'Content for Bob\'s fifth blog.',
    2
  );

INSERT INTO
  comment (content, user_id, blog_id)
VALUES
  ('Great post!', 2, 1),
  ('Thanks for sharing!', 1, 6),
  ('Interesting perspective!', 2, 2),
  ('I agree with this!', 1, 7),
  ('Nice blog!', 2, 3),
  ('Well written!', 1, 8);