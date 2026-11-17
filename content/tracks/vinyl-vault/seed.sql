-- Hand-written for Phase 2A; `cargo xtask gen-seed` replaces it in 2D.

INSERT INTO artists (id, name, country) VALUES
    (1, 'Static Orchard', 'UK'),
    (2, 'Marisol Okafor', 'Nigeria'),
    (3, 'Hollow Pine Collective', 'USA'),
    (4, 'Kestrel Tides', 'Ireland'),
    (5, 'Nadia Varga Quartet', 'Hungary'),
    (6, 'Low Orbit Choir', NULL),
    (7, 'Juniper Vale', 'Canada'),
    (8, 'Lantern Ferry', 'Japan'),
    (9, 'Sundial Sound System', NULL),
    (10, 'Ottoline Reyes', 'Mexico');

INSERT INTO albums (id, artist_id, title, year, genre) VALUES
    (1, 1, 'Copper Skies', 1968, 'Rock'),
    (2, 1, 'Night Bus Hymns', 1971, 'Rock'),
    (3, 2, 'Lagos After Rain', 1974, 'Afrobeat'),
    (4, 2, 'Brass Weather', 1979, 'Afrobeat'),
    (5, 3, 'Timber and Smoke', 1972, 'Folk'),
    (6, 3, 'Long Porch Evenings', 1975, 'Folk'),
    (7, 4, 'Saltwater Letters', 1981, 'Folk'),
    (8, 5, 'Blue Hour Variations', 1963, 'Jazz'),
    (9, 5, 'Late Set at the Kalina', 1966, 'Jazz'),
    (10, 6, 'Signal from Perigee', 1983, 'Electronic'),
    (11, 7, 'Maple Static', 1988, 'Pop'),
    (12, 7, 'Paper Planets', 1991, 'Pop'),
    (13, 8, 'Harbor Lights at Four', 1978, 'Jazz'),
    (14, 9, 'Equinox Dub', 1985, 'Reggae'),
    (15, 10, 'Cantos del Mercado', 1977, 'Latin'),
    (16, 10, 'Marigold Nights', 1986, 'Latin'),
    (17, 4, 'Gull Song', 1969, 'Folk'),
    (18, 6, 'Static Bloom', 1995, 'Electronic');

INSERT INTO pressings (id, album_id, format, condition, price, in_stock) VALUES
    (1, 1, 'LP', 'Very Good', 38.5, 1),
    (2, 1, 'LP', 'Near Mint', 64.0, 0),
    (3, 3, 'LP', 'Good', 22.0, 1),
    (4, 5, 'LP', 'Very Good', 27.5, 1),
    (5, 8, 'LP', 'Near Mint', 85.0, 1),
    (6, 9, 'LP', 'Fair', 12.0, 1),
    (7, 11, '7-inch', 'Very Good', 6.5, 1),
    (8, 12, 'LP', 'Very Good', 19.0, 0),
    (9, 13, 'LP', 'Good', 31.0, 1),
    (10, 14, '12-inch', 'Near Mint', 24.0, 1),
    (11, 16, 'LP', 'Very Good', 17.5, 1),
    (12, 18, 'LP', 'Near Mint', 29.0, 1);
