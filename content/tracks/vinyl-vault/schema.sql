-- Vinyl Vault: a fictional second-hand record shop. Every artist, album
-- and pressing here is made up.

CREATE TABLE artists (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    country TEXT -- NULL when nobody knows where the artist is from
);

CREATE TABLE albums (
    id INTEGER PRIMARY KEY,
    artist_id INTEGER NOT NULL REFERENCES artists (id),
    title TEXT NOT NULL,
    year INTEGER NOT NULL,
    genre TEXT NOT NULL
);

CREATE TABLE pressings (
    id INTEGER PRIMARY KEY,
    album_id INTEGER NOT NULL REFERENCES albums (id),
    format TEXT NOT NULL,
    condition TEXT NOT NULL,
    price REAL NOT NULL,
    in_stock INTEGER NOT NULL
);
