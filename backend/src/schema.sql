CREATE TABLE libraries (
    library_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    provider_id INTEGER NOT NULL,
    native_id TEXT NOT NULL
);

CREATE TABLE albums (
    item_id INTEGER PRIMARY KEY,
    library_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    pic_url TEXT NOT NULL,
    publish_time DATETIME,
    track_count INTEGER,
    description TEXT,
    company TEXT,
    album_type TEXT,
    FOREIGN KEY (library_id) REFERENCES libraries(library_id)
);

CREATE TABLE artists (
    item_id INTEGER PRIMARY KEY,
    library_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    pic_url TEXT NOT NULL,
    description TEXT,
    album_count INTEGER,
    music_count INTEGER,
    alias TEXT,  -- stored as JSON array
    FOREIGN KEY (library_id) REFERENCES libraries(library_id)
);

CREATE TABLE mixes (
    item_id INTEGER PRIMARY KEY,
    library_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    pic_url TEXT NOT NULL,
    track_count INTEGER,
    special_type INTEGER,
    description TEXT,
    create_time DATETIME,
    update_time DATETIME,
    play_count INTEGER,
    user_id INTEGER,
    tags TEXT,  -- stored as JSON array
    FOREIGN KEY (library_id) REFERENCES libraries(library_id)
);

CREATE TABLE album_artist (
    library_id INTEGER NOT NULL,
    album_id TEXT NOT NULL,
    artist_id TEXT NOT NULL,
    edit_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (library_id) REFERENCES libraries(library_id),
    FOREIGN KEY (album_id) REFERENCES albums(item_id),
    FOREIGN KEY (artist_id) REFERENCES artists(item_id),
    UNIQUE(album_id, artist_id, library_id)
);

CREATE TABLE song_artist (
    library_id INTEGER NOT NULL,
    song_id TEXT NOT NULL,
    artist_id TEXT NOT NULL,
    edit_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (library_id) REFERENCES libraries(library_id),
    FOREIGN KEY (song_id) REFERENCES songs(item_id),
    FOREIGN KEY (artist_id) REFERENCES artists(item_id),
    UNIQUE(song_id, artist_id, library_id)
);

CREATE TABLE mix_song (
    library_id INTEGER NOT NULL,
    song_id TEXT NOT NULL,
    mix_id TEXT NOT NULL,
    order_idx INTEGER,
    removed INTEGER DEFAULT 0,
    edit_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (library_id) REFERENCES libraries(library_id),
    FOREIGN KEY (song_id) REFERENCES songs(item_id),
    FOREIGN KEY (mix_id) REFERENCES mixes(item_id),
    UNIQUE(song_id, mix_id, library_id)
);
