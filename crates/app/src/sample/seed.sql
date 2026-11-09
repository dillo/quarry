-- The built-in sample database: a small space agency.
CREATE TABLE spacecraft (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  crew_capacity INTEGER NOT NULL
);
INSERT INTO spacecraft (id, name, crew_capacity) VALUES
  (1, 'Kestrel', 3), (2, 'Heron', 4), (3, 'Albatross', 7), (4, 'Wren', 2);

CREATE TABLE astronauts (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  nationality TEXT NOT NULL,
  joined INTEGER NOT NULL,
  call_sign TEXT
);
INSERT INTO astronauts (id, name, nationality, joined, call_sign) VALUES
  (1, 'Amara Okafor', 'Nigeria', 2019, 'Comet'),
  (2, 'Kenji Watanabe', 'Japan', 2017, NULL),
  (3, 'Lucía Fernández', 'Argentina', 2021, 'Pampa'),
  (4, 'Ingrid Solberg', 'Norway', 2016, 'Fjord'),
  (5, 'Ravi Menon', 'India', 2018, NULL),
  (6, 'Chloé Martin', 'France', 2020, 'Mistral'),
  (7, 'Tomasz Nowak', 'Poland', 2022, NULL),
  (8, 'Aiyana Redcloud', 'United States', 2015, 'Hawk'),
  (9, 'Mei Lin', 'Singapore', 2019, 'Lotus'),
  (10, 'Kwame Mensah', 'Ghana', 2023, NULL),
  (11, 'Sofia Rossi', 'Italy', 2018, 'Vesta'),
  (12, 'Jonas Becker', 'Germany', 2016, NULL);

CREATE TABLE missions (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  launched TEXT NOT NULL,
  spacecraft_id INTEGER NOT NULL REFERENCES spacecraft (id),
  outcome TEXT NOT NULL CHECK (outcome IN ('success', 'partial', 'aborted'))
);
INSERT INTO missions (id, name, launched, spacecraft_id, outcome) VALUES
  (1, 'Dawn Relay', '2020-03-14', 1, 'success'),
  (2, 'Cold Harbor', '2020-09-02', 2, 'success'),
  (3, 'Tidewatch', '2021-01-21', 1, 'partial'),
  (4, 'Long Meridian', '2021-07-30', 3, 'success'),
  (5, 'Glass Orchard', '2022-02-11', 2, 'aborted'),
  (6, 'Northern Choir', '2022-10-05', 3, 'success'),
  (7, 'Quiet Lantern', '2023-04-18', 4, 'success'),
  (8, 'Ember Field', '2023-11-27', 3, 'partial'),
  (9, 'Salt Comet', '2024-06-09', 2, 'success'),
  (10, 'Far Orchard', '2025-02-03', 3, 'success');

CREATE TABLE crew_assignments (
  mission_id INTEGER NOT NULL REFERENCES missions (id),
  astronaut_id INTEGER NOT NULL REFERENCES astronauts (id),
  role TEXT NOT NULL,
  PRIMARY KEY (mission_id, astronaut_id)
);
INSERT INTO crew_assignments (mission_id, astronaut_id, role) VALUES
  (1, 1, 'commander'), (1, 4, 'pilot'), (1, 8, 'engineer'),
  (2, 2, 'commander'), (2, 5, 'pilot'), (2, 9, 'scientist'), (2, 12, 'engineer'),
  (3, 4, 'commander'), (3, 6, 'pilot'),
  (4, 8, 'commander'), (4, 1, 'pilot'), (4, 3, 'scientist'), (4, 11, 'engineer'),
  (4, 5, 'medic'), (4, 9, 'scientist'),
  (5, 2, 'commander'), (5, 7, 'pilot'),
  (6, 1, 'commander'), (6, 4, 'pilot'), (6, 6, 'scientist'), (6, 8, 'engineer'),
  (7, 9, 'commander'), (7, 3, 'pilot'),
  (8, 11, 'commander'), (8, 1, 'pilot'), (8, 10, 'engineer'), (8, 12, 'scientist'),
  (9, 5, 'commander'), (9, 6, 'pilot'), (9, 4, 'engineer'),
  (10, 8, 'commander'), (10, 1, 'pilot'), (10, 9, 'scientist'), (10, 3, 'medic');

-- 200,000 readings, so streaming and cancelling have something to chew on.
CREATE TABLE telemetry (
  id INTEGER PRIMARY KEY,
  mission_id INTEGER NOT NULL REFERENCES missions (id),
  seconds INTEGER NOT NULL,
  altitude_km REAL NOT NULL,
  velocity_kms REAL NOT NULL
);
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 200000)
INSERT INTO telemetry (id, mission_id, seconds, altitude_km, velocity_kms)
SELECT i, 1 + (i % 10), i / 10, round(400 + (i % 997) * 0.25, 2), round(7.6 + (i % 113) * 0.001, 3)
FROM n;

CREATE VIEW mission_crews AS
SELECT m.name AS mission, a.name AS astronaut, c.role
FROM crew_assignments c
JOIN missions m ON m.id = c.mission_id
JOIN astronauts a ON a.id = c.astronaut_id;
